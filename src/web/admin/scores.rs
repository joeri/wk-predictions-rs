use models::{
    Favourite, Match, MatchOutcome, MatchParticipant, MatchPrediction, MatchWithParticipants, User,
};
use scores::user_match_points;
use web::app_state::DbExecutor;

use actix::prelude::*;
use actix_web::{AsyncResponder, Either, HttpResponse, Responder, State};
use diesel::{self, prelude::*};
use failure;
use futures::Future;
use web::{app_state::AppState, auth::CurrentUser};

struct RecalculateScores;

impl Message for RecalculateScores {
    type Result = Result<(), failure::Error>;
}

impl Handler<RecalculateScores> for DbExecutor {
    type Result = Result<(), failure::Error>;

    fn handle(&mut self, _msg: RecalculateScores, _ctx: &mut Self::Context) -> Self::Result {
        use diesel::{insert_into, update};

        Ok(self.connection
            .transaction::<(), diesel::result::Error, _>(|| {
                let games = {
                    use schema::{match_participants, matches};
                    use std::collections::HashMap;

                    let plain_games = matches::table.load::<Match>(&self.connection)?;
                    let match_participants =
                        match_participants::table.load::<MatchParticipant>(&self.connection)?;

                    let match_outcomes = {
                        use schema::match_outcomes::dsl::*;

                        match_outcomes
                            .select((
                                match_id,
                                home_score,
                                away_score,
                                time_of_first_goal,
                                home_penalties,
                                away_penalties,
                                duration,
                            ))
                            .load::<MatchOutcome>(&self.connection)?
                    };

                    let participants_by_id = match_participants
                        .into_iter()
                        .map(|p| (p.match_participant_id, p))
                        .collect::<HashMap<_, _>>();

                    plain_games
                        .into_iter()
                        .map(|game| {
                            let match_outcome = match_outcomes
                                .iter()
                                .find(|outcome| outcome.match_id == game.match_id);

                            (
                                MatchWithParticipants {
                                    match_id: game.match_id,
                                    stage_id: game.stage_id,
                                    home_participant: participants_by_id[&game.home_participant_id]
                                        .clone(),
                                    away_participant: participants_by_id[&game.away_participant_id]
                                        .clone(),
                                    time: game.time,
                                },
                                match_outcome,
                            )
                        })
                        .filter(|(_a, b)| b.is_some())
                        .map(|(a, b)| (a, b.unwrap().clone()))
                        .collect::<Vec<_>>()
                };

                let users = {
                    use schema::users::dsl::*;

                    users.load::<User>(&self.connection)?
                };
                let mut match_points = Vec::with_capacity(users.len() * games.len());

                let match_predictions = MatchPrediction::belonging_to(&users)
                    .load::<MatchPrediction>(&self.connection)?
                    .grouped_by(&users);
                let favourites = Favourite::belonging_to(&users)
                    .load::<Favourite>(&self.connection)?
                    .grouped_by(&users);

                let users_with_everything = users
                    .into_iter()
                    .zip(match_predictions)
                    .zip(favourites)
                    .map(|((a, b), c)| (a, b, c))
                    .collect::<Vec<_>>();

                for user in users_with_everything {
                    for game in &games {
                        let prediction = user.1
                            .iter()
                            .find(|prediction| prediction.match_id == game.0.match_id)
                            .cloned();
                        match_points.push(user_match_points(
                            &(user.0.clone(), prediction, user.2.clone()),
                            game,
                        ));
                    }
                }

                {
                    use diesel::pg::upsert::excluded;
                    use schema::user_match_points::dsl::*;

                    insert_into(user_match_points)
                        .values(&match_points)
                        .on_conflict((user_id, match_id))
                        .do_update()
                        .set((
                            prediction.eq(excluded(prediction)),
                            favourites.eq(excluded(favourites)),
                            time_of_first_goal.eq(excluded(time_of_first_goal)),
                            total.eq(excluded(total)),
                        ))
                        .execute(&self.connection)?;
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

pub fn recalculate((auth, state): (CurrentUser, State<AppState>)) -> impl Responder {
    if auth.current_user.user_id == 1 {
        Either::A(
            state
                .db
                .send(RecalculateScores)
                .and_then(move |data| match data {
                    Ok(()) => Ok(HttpResponse::SeeOther()
                        .header("Location", "/admin/matches")
                        .finish()),
                    Err(error) => {
                        println!("{:?}", error);
                        Ok(HttpResponse::SeeOther().header("Location", "/").finish())
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
