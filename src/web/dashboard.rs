use models::{MatchOutcome, MatchPrediction, User};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::{AppState, DbExecutor};

use actix::prelude::*;
use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, Either, Error, FutureResponse, HttpRequest, HttpResponse, State};
use futures::Future;

use diesel::prelude::*;

use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, QueryableByName)]
struct MatchWithAllInfo {
    #[sql_type = "diesel::sql_types::Integer"]
    match_id: i32,
    #[sql_type = "diesel::sql_types::Integer"]
    location_id: i32,
    #[sql_type = "diesel::sql_types::Timestamptz"]
    time: DateTime<Utc>,

    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Integer>"]
    home_group_id: Option<i32>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Integer>"]
    home_group_drawn_place: Option<i32>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Integer>"]
    home_previous_match_id: Option<i32>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Varchar>"]
    home_previous_match_result: Option<String>,

    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Varchar>"]
    home_country_name: Option<String>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Varchar>"]
    home_country_flag: Option<String>,

    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Integer>"]
    away_group_id: Option<i32>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Integer>"]
    away_group_drawn_place: Option<i32>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Integer>"]
    away_previous_match_id: Option<i32>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Varchar>"]
    away_previous_match_result: Option<String>,

    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Varchar>"]
    away_country_name: Option<String>,
    #[sql_type = "diesel::sql_types::Nullable<diesel::sql_types::Varchar>"]
    away_country_flag: Option<String>,
}

struct DashboardData {
    current_user: User,
    leader_board: Vec<User>,
    upcoming: Vec<(MatchWithAllInfo, Option<MatchPrediction>)>,
    finished: Vec<(
        MatchWithAllInfo,
        Option<MatchOutcome>,
        Option<MatchPrediction>,
    )>,
}

struct FetchDataForDashboard {
    user_id: i32,
}

impl Message for FetchDataForDashboard {
    type Result = Result<DashboardData, Error>;
}

fn fetch_current_user(db: &DbExecutor, current_user_id: i32) -> User {
    use schema::users::dsl::*;

    users
        .filter(user_id.eq(current_user_id))
        .first(&db.connection)
        .expect("Couldn't fetch leaderboard")
}

fn fetch_users(db: &DbExecutor, amount: i64) -> Vec<User> {
    use schema::users::dsl::*;

    users
        .order(score.asc())
        .limit(amount)
        .get_results(&db.connection)
        .expect("Couldn't fetch leaderboard")
}

fn fetch_upcoming(
    db: &DbExecutor,
    current_user_id: i32,
    amount: i32,
) -> Vec<(MatchWithAllInfo, Option<MatchPrediction>)> {
    use diesel::sql_query;
    use diesel::sql_types::{Integer, Timestamptz};

    let match_infos = sql_query(include_str!(
        "../queries/dashboard/fetch_upcoming_matches.sql"
    )).bind::<Timestamptz, _>(Utc::now())
        .bind::<Integer, _>(amount)
        .get_results(&db.connection)
        .expect("Couldn't fetch upcoming matches");

    let predictions_grouped_by_match_infos = {
        use schema::match_predictions::dsl::*;
        use std::collections::HashMap;

        let match_ids = match_infos
            .iter()
            .map(|match_info: &MatchWithAllInfo| match_info.match_id)
            .collect::<Vec<_>>();
        let predictions = match_predictions
            .filter(user_id.eq(current_user_id))
            .filter(match_id.eq_any(match_ids))
            .get_results::<MatchPrediction>(&db.connection)
            .expect("Couldn't fetch predictions");

        let id_indices: HashMap<_, _> = match_infos
            .iter()
            .enumerate()
            .map(|(i, u)| (u.match_id, i))
            .collect();

        let mut result = match_infos.iter().map(|_| None).collect::<Vec<_>>();
        for child in predictions {
            let index = id_indices[&child.match_id];
            result[index] = Some(child);
        }

        result
    };

    match_infos
        .into_iter()
        .zip(predictions_grouped_by_match_infos)
        .collect()
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
            current_user: fetch_current_user(&self, msg.user_id),
            leader_board: fetch_users(&self, 10),
            upcoming: fetch_upcoming(&self, msg.user_id, 10),
            finished: Vec::new(),
        })
    }
}

pub fn index(
    request: HttpRequest<AppState>,
    state: State<AppState>,
) -> Either<FutureResponse<HttpResponse>, HttpResponse> {
    match request.identity() {
        Some(current_user_id) => {
            println!("We have a user identity {:?}", current_user_id);
            Either::A(
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

                                let rendered = TEMPLATE_SERVICE.render("dashboard.html", &context);
                                match rendered {
                                    Ok(body) => {
                                        HttpResponse::Ok().content_type("text/html").body(body)
                                    }
                                    Err(error) => {
                                        println!("{:?}", error);
                                        HttpResponse::InternalServerError()
                                            .content_type("text/html")
                                            .body("Something went wrong")
                                    }
                                }
                            }
                            Err(_) => HttpResponse::InternalServerError().into(),
                        })
                    })
                    .responder(),
            )
        }
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
