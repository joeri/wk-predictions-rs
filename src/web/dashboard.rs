use models::{Country, Favourite, MatchOutcome, MatchPrediction, MatchWithAllInfo, User};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::{AppState, DbExecutor};

use actix::prelude::*;
use actix_web::{middleware::identity::RequestIdentity, AsyncResponder, Either, Error,
                FutureResponse, HttpRequest, HttpResponse, State};
use futures::Future;

use diesel::prelude::*;
use failure;

use chrono::Utc;

struct DashboardData {
    current_user: User,
    leader_board: Vec<User>,
    upcoming: Vec<(MatchWithAllInfo, Option<MatchPrediction>)>,
    finished: Vec<(
        MatchWithAllInfo,
        Option<MatchOutcome>,
        Option<MatchPrediction>,
    )>,
    favourites: Vec<(Favourite, Option<Country>)>,
}

struct FetchDataForDashboard {
    user_id: i32,
}

impl Message for FetchDataForDashboard {
    type Result = Result<DashboardData, Error>;
}

fn fetch_current_user(db: &DbExecutor, current_user_id: i32) -> Result<User, failure::Error> {
    use schema::users::dsl::*;

    Ok(users
        .filter(user_id.eq(current_user_id))
        .first(&db.connection)?)
}

fn fetch_users(db: &DbExecutor, amount: i64) -> Result<Vec<User>, failure::Error> {
    use schema::users::dsl::*;

    Ok(users
        .order(score.asc())
        .limit(amount)
        .get_results(&db.connection)?)
}

fn fetch_upcoming(
    db: &DbExecutor,
    current_user_id: i32,
    amount: i64,
) -> Result<Vec<(MatchWithAllInfo, Option<MatchPrediction>)>, failure::Error> {
    let match_infos = {
        use schema::full_match_infos::dsl::*;

        full_match_infos
            .filter(time.gt(Utc::now()))
            .order(time.asc())
            .limit(amount)
            .load::<MatchWithAllInfo>(&db.connection)?
    };

    use std::collections::HashMap;

    let match_ids = match_infos
        .iter()
        .map(|match_info: &MatchWithAllInfo| match_info.match_id)
        .collect::<Vec<_>>();

    let id_indices: HashMap<_, _> = match_infos
        .iter()
        .enumerate()
        .map(|(i, u)| (u.match_id, i))
        .collect();

    let predictions_grouped_by_match_infos = {
        use schema::match_predictions::dsl::*;
        let predictions = match_predictions
            .filter(user_id.eq(current_user_id))
            .filter(match_id.eq_any(&match_ids))
            .get_results::<MatchPrediction>(&db.connection)?;

        let mut result = match_infos.iter().map(|_| None).collect::<Vec<_>>();
        for child in predictions {
            let index = id_indices[&child.match_id];
            result[index] = Some(child);
        }

        result
    };

    Ok(match_infos
        .into_iter()
        .zip(predictions_grouped_by_match_infos)
        .collect())
}

fn fetch_previous(
    db: &DbExecutor,
    current_user_id: i32,
    amount: i64,
) -> Result<
    Vec<(
        MatchWithAllInfo,
        Option<MatchOutcome>,
        Option<MatchPrediction>,
    )>,
    failure::Error,
> {
    use std::collections::HashMap;

    let match_infos = {
        use schema::full_match_infos::dsl::*;

        full_match_infos
            .filter(time.le(Utc::now()))
            .order(time.desc())
            .limit(amount)
            .load::<MatchWithAllInfo>(&db.connection)?
    };

    let match_ids = match_infos
        .iter()
        .map(|match_info: &MatchWithAllInfo| match_info.match_id)
        .collect::<Vec<_>>();

    let id_indices: HashMap<_, _> = match_infos
        .iter()
        .enumerate()
        .map(|(i, u)| (u.match_id, i))
        .collect();

    let predictions_grouped_by_match_infos = {
        use schema::match_predictions::dsl::*;

        let predictions = match_predictions
            .filter(user_id.eq(current_user_id))
            .filter(match_id.eq_any(&match_ids))
            .load::<MatchPrediction>(&db.connection)?;

        let mut result = match_infos.iter().map(|_| None).collect::<Vec<_>>();
        for child in predictions {
            let index = id_indices[&child.match_id];
            result[index] = Some(child);
        }

        result
    };

    let results_grouped_by_match_infos = {
        use schema::match_outcomes::dsl::*;

        let outcomes = match_outcomes
            .filter(match_id.eq_any(&match_ids))
            .select((match_id, home_score, away_score, time_of_first_goal))
            .load::<MatchOutcome>(&db.connection)?;

        let mut result = match_infos.iter().map(|_| None).collect::<Vec<_>>();
        for child in outcomes {
            let index = id_indices[&child.match_id];
            result[index] = Some(child);
        }

        result
    };

    Ok(match_infos
        .into_iter()
        .zip(results_grouped_by_match_infos)
        .zip(predictions_grouped_by_match_infos)
        .map(|((a, b), c)| (a, b, c))
        .collect())
}

fn fetch_favourites(
    db: &DbExecutor,
    current_user_id: i32,
) -> Result<Vec<(Favourite, Option<Country>)>, failure::Error> {
    let mut current_selection = {
        use schema::countries;
        use schema::favourites::dsl::*;

        favourites
            .filter(user_id.eq(current_user_id))
            .order(choice)
            .left_join(countries::table)
            .load(&db.connection)?
    };

    if current_selection.len() < 4 {
        for i in (current_selection.len() + 1)..=4 {
            current_selection.push((
                Favourite {
                    user_id: current_user_id,
                    country_id: None,
                    choice: i as i16,
                    created_at: Utc::now().naive_local(), // Doesn't matter too much if this is the right method (as opposed to naive_utc)
                    updated_at: Utc::now().naive_local(),
                    phase: 0,
                    source: "manual".to_string(),
                },
                None,
            ));
        }
    }

    Ok(current_selection)
}

impl Handler<FetchDataForDashboard> for DbExecutor {
    type Result = Result<DashboardData, Error>;

    fn handle(&mut self, msg: FetchDataForDashboard, _: &mut Self::Context) -> Self::Result {
        // For now we return
        // Top 10 users (by score)
        // Most recent 6(?) matches, with outcomes (if already present) and predictions for current
        // player
        // 6 following upcoming matches, with predictions for current player

        Ok(DashboardData {
            current_user: fetch_current_user(&self, msg.user_id)?,
            leader_board: fetch_users(&self, 10)?,
            upcoming: fetch_upcoming(&self, msg.user_id, 10)?,
            finished: fetch_previous(&self, msg.user_id, 10)?,
            favourites: fetch_favourites(&self, msg.user_id)?,
        })
    }
}

pub fn index(
    request: HttpRequest<AppState>,
    state: State<AppState>,
) -> Either<FutureResponse<HttpResponse>, HttpResponse> {
    match request.identity() {
        Some(current_user_id) => Either::A(
            state
                .db
                .send(FetchDataForDashboard {
                    user_id: current_user_id.parse().unwrap(),
                })
                .from_err()
                .and_then(move |res| {
                    Ok(match res {
                        Ok(dashboard_data) => {
                            let mut context = Context::new();
                            context.add("current_user", &dashboard_data.current_user);
                            context.add("leader_board", &dashboard_data.leader_board);
                            context.add("upcoming", &dashboard_data.upcoming);
                            context.add("finished", &dashboard_data.finished);
                            context.add("favourites", &dashboard_data.favourites);

                            let rendered = TEMPLATE_SERVICE.render("dashboard.html", &context);
                            match rendered {
                                Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
                                Err(error) => {
                                    println!("{:?}", error);
                                    HttpResponse::InternalServerError()
                                        .content_type("text/html")
                                        .body("Something went wrong")
                                }
                            }
                        }
                        Err(_) => HttpResponse::InternalServerError()
                            .content_type("text/html")
                            .body("Something went very wrong"),
                    })
                })
                .responder(),
        ),
        None => {
            let mut context = Context::new();

            let rendered = TEMPLATE_SERVICE.render("unauthenticated.html", &context);
            Either::B(match rendered {
                Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
                Err(error) => {
                    println!("{}", error);
                    HttpResponse::InternalServerError()
                        .content_type("text/html")
                        .body("Something went wrong")
                }
            })
        }
    }
}
