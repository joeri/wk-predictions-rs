use models::{Favourite, Match, MatchOutcome, MatchPrediction, MatchWithAllInfo,
             MatchWithParticipants, User};
use scores::user_match_points;
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::DbExecutor;

use actix::prelude::*;
use actix_web::{AsyncResponder, Either, Form, HttpResponse, Path, Responder, State};
use chrono::Utc;
use diesel::{self, prelude::*};
use failure;
use futures::Future;
use web::{app_state::AppState, auth::CurrentUser};

struct IndexMatchOutcomes;

impl Message for IndexMatchOutcomes {
    type Result = Result<Vec<(MatchWithAllInfo, Option<MatchOutcome>)>, failure::Error>;
}

impl Handler<IndexMatchOutcomes> for DbExecutor {
    type Result = Result<Vec<(MatchWithAllInfo, Option<MatchOutcome>)>, failure::Error>;

    fn handle(&mut self, msg: IndexMatchOutcomes, _ctx: &mut Self::Context) -> Self::Result {
        use schema::full_match_infos::dsl::*;
        use schema::match_outcomes;

        Ok(full_match_infos
            .filter(time.le(Utc::now()))
            .left_join(match_outcomes::table.on(match_outcomes::columns::match_id.eq(match_id)))
            .select((
                full_match_infos::all_columns(),
                (
                    match_outcomes::columns::match_id,
                    match_outcomes::columns::home_score,
                    match_outcomes::columns::away_score,
                    match_outcomes::columns::time_of_first_goal,
                ).nullable(),
            ))
            .order((time.desc(), match_id.asc()))
            .load::<(MatchWithAllInfo, Option<MatchOutcome>)>(&self.connection)?)
    }
}

pub fn index((auth, state): (CurrentUser, State<AppState>)) -> impl Responder {
    if auth.current_user.user_id == 1 {
        Either::A(
            state
                .db
                .send(IndexMatchOutcomes)
                .and_then(move |match_outcomes| match match_outcomes {
                    Ok(matches) => {
                        let mut context = Context::new();
                        context.add("current_user", &auth.current_user);
                        context.add("matches", &matches);
                        let rendered =
                            TEMPLATE_SERVICE.render("admin/matches/index.html", &context);

                        match rendered {
                            Ok(body) => Ok(HttpResponse::Ok().content_type("text/html").body(body)),
                            Err(error) => {
                                println!("{:?}", error);
                                Ok(HttpResponse::InternalServerError()
                                    .content_type("text/html")
                                    .body("Something went wrong"))
                            }
                        }
                    }
                    Err(_) => Ok(HttpResponse::InternalServerError()
                        .content_type("text/html")
                        .body("Something went wrong")),
                })
                .responder(),
        )
    } else {
        Either::B(
            HttpResponse::Forbidden()
                .content_type("text/html")
                .body("You do not have permission to view this page"),
        )
    }
}

struct FetchMatchOutcomeInfo {
    match_id: i32,
}

impl Message for FetchMatchOutcomeInfo {
    type Result = Result<(MatchWithAllInfo, Option<MatchOutcome>), failure::Error>;
}

impl Handler<FetchMatchOutcomeInfo> for DbExecutor {
    type Result = Result<(MatchWithAllInfo, Option<MatchOutcome>), failure::Error>;

    fn handle(&mut self, msg: FetchMatchOutcomeInfo, _ctx: &mut Self::Context) -> Self::Result {
        use schema::full_match_infos::dsl::*;
        use schema::match_outcomes;

        Ok(full_match_infos
            .filter(time.le(Utc::now()))
            .filter(match_id.eq(msg.match_id))
            .left_join(match_outcomes::table.on(match_outcomes::columns::match_id.eq(match_id)))
            .select((
                full_match_infos::all_columns(),
                (
                    match_outcomes::columns::match_id,
                    match_outcomes::columns::home_score,
                    match_outcomes::columns::away_score,
                    match_outcomes::columns::time_of_first_goal,
                ).nullable(),
            ))
            .first::<(MatchWithAllInfo, Option<MatchOutcome>)>(&self.connection)?)
    }
}

pub fn edit((auth, path, state): (CurrentUser, Path<(i32,)>, State<AppState>)) -> impl Responder {
    if auth.current_user.user_id == 1 {
        Either::A(
            state
                .db
                .send(FetchMatchOutcomeInfo { match_id: path.0 })
                .and_then(move |result| match result {
                    Ok((game, outcome)) => {
                        let mut context = Context::new();
                        context.add("current_user", &auth.current_user);
                        context.add("match", &game);
                        context.add("outcome", &outcome);
                        let rendered = TEMPLATE_SERVICE.render("admin/matches/edit.html", &context);

                        match rendered {
                            Ok(body) => Ok(HttpResponse::Ok().content_type("text/html").body(body)),
                            Err(error) => {
                                println!("{:?}", error);
                                Ok(HttpResponse::InternalServerError()
                                    .content_type("text/html")
                                    .body("Something went wrong"))
                            }
                        }
                    }
                    Err(error) => {
                        println!("{:?}", error);
                        Ok(HttpResponse::InternalServerError()
                            .content_type("text/html")
                            .body("Something went wrong"))
                    }
                })
                .responder(),
        )
    } else {
        Either::B(
            HttpResponse::Forbidden()
                .content_type("text/html")
                .body("You do not have permission to view this page"),
        )
    }
}

struct UpdateMatchOutcomeInfo {
    outcome: MatchOutcome,
}

impl Message for UpdateMatchOutcomeInfo {
    type Result = Result<(), failure::Error>;
}

impl Handler<UpdateMatchOutcomeInfo> for DbExecutor {
    type Result = Result<(), failure::Error>;

    fn handle(&mut self, msg: UpdateMatchOutcomeInfo, _ctx: &mut Self::Context) -> Self::Result {
        // First start a transaction
        // Create the match outcome, or replace it
        // Then go over all users (we only have a few, so should be doable), and calculate the
        // UserMatchPoints, insert/replace on conflict
        // Sum the scores for all users at the end
        // commit
        use diesel::{insert_into, update};

        Ok(self.connection
            .transaction::<(), diesel::result::Error, _>(|| {
                {
                    use schema::match_outcomes::dsl::*;

                    insert_into(match_outcomes)
                        .values(&msg.outcome)
                        .on_conflict(match_id)
                        .do_update()
                        .set(&msg.outcome)
                        .execute(&self.connection)?;
                }
                let game = {
                    let game = {
                        use schema::matches::dsl::*;

                        matches
                            .filter(match_id.eq(msg.outcome.match_id))
                            .first::<Match>(&self.connection)?
                    };

                    let home_participant = {
                        use schema::match_participants::dsl::*;

                        match_participants
                            .filter(match_participant_id.eq(game.home_participant_id))
                            .first(&self.connection)?
                    };

                    let away_participant = {
                        use schema::match_participants::dsl::*;

                        match_participants
                            .filter(match_participant_id.eq(game.away_participant_id))
                            .first(&self.connection)?
                    };

                    (
                        MatchWithParticipants {
                            match_id: game.match_id,
                            home_participant,
                            away_participant,
                            time: game.time,
                        },
                        msg.outcome.clone(),
                    )
                };

                let users_with_prediction =
                    {
                        use schema::match_predictions;
                        use schema::users::dsl::*;

                        users
                            .left_join(match_predictions::table.on(
                                match_predictions::columns::user_id.eq(user_id).and(
                                    match_predictions::columns::match_id.eq(msg.outcome.match_id),
                                ),
                            ))
                            .load::<(User, Option<MatchPrediction>)>(&self.connection)?
                    };
                let users = users_with_prediction
                    .iter()
                    .map(|u| u.0.clone())
                    .collect::<Vec<_>>();
                let favourites = Favourite::belonging_to(&users)
                    .load::<Favourite>(&self.connection)?
                    .grouped_by(&users);
                let users_with_everything = users_with_prediction
                    .into_iter()
                    .zip(favourites)
                    .map(|((a, b), c)| (a, b, c))
                    .collect::<Vec<_>>();

                for user in users_with_everything {
                    let points = user_match_points(&user, &game);
                    {
                        use schema::user_match_points::dsl::*;

                        insert_into(user_match_points)
                            .values(&points)
                            .on_conflict((user_id, match_id))
                            .do_update()
                            .set(&points)
                            .execute(&self.connection)?;
                    }
                }

                {
                    use diesel::dsl::sql;
                    use schema::users::dsl::*;

                    update(users)
                        .set(score.eq(sql("(SELECT sum(user_match_points.total) FROM user_match_points WHERE user_match_points.user_id = users.user_id)")))
                        .execute(&self.connection)?;
                }

                Ok(())
            })?)
    }
}

pub fn update(
    (auth, outcome, state): (CurrentUser, Form<MatchOutcome>, State<AppState>),
) -> impl Responder {
    if auth.current_user.user_id == 1 {
        Either::A(
            state
                .db
                .send(UpdateMatchOutcomeInfo {
                    outcome: outcome.clone(),
                })
                .and_then(move |data| match data {
                    Ok(()) => Ok(HttpResponse::SeeOther()
                        .header("Location", "/admin/matches")
                        .finish()),
                    Err(error) => {
                        println!("{:?}", error);
                        Ok(HttpResponse::SeeOther()
                            .header("Location", format!("/admin/matches/{}", outcome.match_id))
                            .finish())
                    }
                })
                .responder(),
        )
    } else {
        Either::B(
            HttpResponse::Forbidden()
                .content_type("text/html")
                .body("You do not have permission to view this page"),
        )
    }
}
