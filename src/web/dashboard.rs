use models::{Match, MatchOutcome, MatchParticipant, MatchPrediction, User};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::{AppState, DbExecutor};

use actix::prelude::*;
use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, Either, Error, FutureResponse, HttpRequest, HttpResponse, State};
use futures::Future;

use diesel::prelude::*;

struct DashboardData {
    leader_board: Vec<User>,
    finished: Vec<(
        Match,
        MatchParticipant,
        MatchParticipant,
        Option<MatchPrediction>,
    )>,
    upcoming: Vec<(
        Match,
        MatchParticipant,
        MatchParticipant,
        Option<MatchPrediction>,
        Option<MatchOutcome>,
    )>,
}

struct FetchDataForDashboard {
    user_id: i32,
}

impl Message for FetchDataForDashboard {
    type Result = Result<DashboardData, Error>;
}

impl Handler<FetchDataForDashboard> for DbExecutor {
    type Result = Result<DashboardData, Error>;

    fn handle(&mut self, _msg: FetchDataForDashboard, _: &mut Self::Context) -> Self::Result {
        use schema::users::dsl::*;

        // For now we return
        // Top 10 users (by score)
        // Most recent 6(?) matches, with outcomes (if already present) and predictions for current
        // player
        // 6 following upcoming matches, with predictions for current player

        // normal diesel operations
        let leaders = users
            .order(score.asc())
            .limit(10)
            .get_results(&self.connection)
            .expect("Couldn't fetch leaderboard");

        Ok(DashboardData {
            leader_board: leaders,
            finished: Vec::new(),
            upcoming: Vec::new(),
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
                                context.add("leader_board", &dashboard_data.leader_board);
                                context.add("upcoming", &dashboard_data.upcoming);
                                context.add("finished", &dashboard_data.finished);

                                let rendered = TEMPLATE_SERVICE.render("dashboard.html", &context);
                                match rendered {
                                    Ok(body) => {
                                        HttpResponse::Ok().content_type("text/html").body(body)
                                    }
                                    Err(error) => {
                                        println!("{}", error);
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
