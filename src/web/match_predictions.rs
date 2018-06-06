use models::{Location, Match, MatchOutcome, MatchPrediction, MatchWithAllInfo, UpdatedPrediction};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::DbExecutor;
use web::{app_state::AppState, auth::CurrentUser};

use actix::prelude::*;
use actix_web::{AsyncResponder, Form, FutureResponse, HttpRequest, HttpResponse, Path};
use chrono::Utc;
use diesel::prelude::*;
use failure;
use futures::Future;

use std::{error::Error as StdError, fmt};

struct FetchPredictionInfo {
    user_id: i32,
    match_id: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct PredictionInfo {
    match_with_info: MatchWithAllInfo,
    location: Location,
    prediction: Option<MatchPrediction>,
    outcome: Option<MatchOutcome>,
}

impl PredictionInfo {
    fn in_future(&self) -> bool {
        self.match_with_info.time > Utc::now()
    }
}

impl Message for FetchPredictionInfo {
    type Result = Result<PredictionInfo, failure::Error>;
}

impl Handler<FetchPredictionInfo> for DbExecutor {
    type Result = Result<PredictionInfo, failure::Error>;

    fn handle(&mut self, msg: FetchPredictionInfo, _: &mut Self::Context) -> Self::Result {
        let match_info = {
            use schema::full_match_infos::dsl::*;

            full_match_infos
                .filter(match_id.eq(msg.match_id))
                .first::<MatchWithAllInfo>(&self.connection)?
        };

        let prediction = {
            use schema::match_predictions::dsl::*;

            match_predictions
                .filter(user_id.eq(msg.user_id))
                .filter(match_id.eq(match_info.match_id))
                .first(&self.connection)
                .optional()?
        };

        let location = {
            use schema::locations::dsl::*;

            locations
                .filter(location_id.eq(match_info.location_id))
                .first(&self.connection)?
        };

        Ok(PredictionInfo {
            match_with_info: match_info,
            prediction,
            location,
            outcome: None,
        })
    }
}

pub fn edit(
    auth: CurrentUser,
    path: Path<(i32,)>,
    req: HttpRequest<AppState>,
) -> FutureResponse<HttpResponse> {
    req.state()
        .db
        .send(FetchPredictionInfo {
            user_id: auth.current_user.user_id,
            match_id: path.0,
        })
        .from_err()
        .and_then(move |prediction_info| {
            Ok(match prediction_info {
                Ok(info) => {
                    let mut context = Context::new();
                    context.add("current_user", &auth.current_user);
                    context.add("match", &info.match_with_info);
                    context.add("location", &info.location);
                    context.add("prediction", &info.prediction);

                    let rendered = if info.in_future() {
                        TEMPLATE_SERVICE.render("predictions/edit.html", &context)
                    } else {
                        context.add("outcome", &info.outcome);
                        TEMPLATE_SERVICE.render("predictions/show.html", &context)
                    };

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
                _ => HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body("Something went wrong"),
            })
        })
        .responder()
}

#[derive(Debug)]
struct TooLateToPredict;

impl fmt::Display for TooLateToPredict {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Too late to predict this match")
    }
}

impl StdError for TooLateToPredict {
    fn description(&self) -> &str {
        "Too late to predict this match"
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PredictionForm {
    home_score: i16,
    away_score: i16,
    time_of_first_goal: i16,
}

struct UpdatePredictionInfo {
    user_id: i32,
    match_id: i32,
    prediction: PredictionForm,
}

impl Message for UpdatePredictionInfo {
    type Result = Result<(), failure::Error>;
}

impl Handler<UpdatePredictionInfo> for DbExecutor {
    type Result = Result<(), failure::Error>;

    fn handle(&mut self, msg: UpdatePredictionInfo, _: &mut Self::Context) -> Self::Result {
        let match_info = {
            use schema::matches::dsl::*;

            matches
                .filter(match_id.eq(msg.match_id))
                .first::<Match>(&self.connection)?
        };

        let prediction = UpdatedPrediction {
            user_id: msg.user_id,
            match_id: msg.match_id,

            home_score: msg.prediction.home_score,
            away_score: msg.prediction.away_score,

            time_of_first_goal: msg.prediction.time_of_first_goal,
        };

        if match_info.time >= Utc::now() {
            use diesel::insert_into;
            use schema::match_predictions::dsl::*;

            insert_into(match_predictions)
                .values(&prediction)
                .on_conflict((match_id, user_id))
                .do_update()
                .set(&prediction)
                .execute(&self.connection)?;
            Ok(())
        } else {
            Err(TooLateToPredict)?
        }
    }
}

pub fn update(
    auth: CurrentUser,
    path_and_form: (Path<(i32,)>, Form<PredictionForm>),
    req: HttpRequest<AppState>,
) -> FutureResponse<HttpResponse> {
    let (path, form) = path_and_form;
    req.state()
        .db
        .send(UpdatePredictionInfo {
            user_id: auth.current_user.user_id,
            match_id: path.0,
            prediction: form.into_inner(),
        })
        .from_err()
        .and_then(|result| match result {
            Ok(_) => Ok(HttpResponse::SeeOther().header("Location", "/").finish()),
            Err(error) => {
                println!("{:?}", error);
                Ok(HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body("Something went wrong"))
            }
        })
        .responder()
}
