use models::{Location, Match, MatchOutcome, MatchPrediction, MatchWithAllInfo, UpdatedPrediction};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::DbExecutor;
use web::{app_state::AppState, auth::CurrentUser};

use actix::prelude::*;
use actix_web::{self, dev::AsyncResult, error::ResponseError, AsyncResponder, Form, FromRequest,
                FutureResponse, HttpRequest, HttpResponse, Path, Responder};
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

struct FetchBulkPredictionInfo {
    user_id: i32,
}

impl Message for FetchBulkPredictionInfo {
    type Result = Result<Vec<(MatchWithAllInfo, Option<MatchPrediction>)>, failure::Error>;
}

impl Handler<FetchBulkPredictionInfo> for DbExecutor {
    type Result = Result<Vec<(MatchWithAllInfo, Option<MatchPrediction>)>, failure::Error>;

    fn handle(&mut self, msg: FetchBulkPredictionInfo, _: &mut Self::Context) -> Self::Result {
        use schema::full_match_infos::dsl::*;
        use schema::match_predictions;

        Ok(full_match_infos
            .left_outer_join(
                match_predictions::table.on(match_id
                    .eq(match_predictions::match_id)
                    .and(match_predictions::user_id.eq(msg.user_id))),
            )
            .filter(home_country_name.is_not_null())
            .filter(away_country_name.is_not_null())
            .filter(time.gt(Utc::now()))
            .order(time.asc())
            .load::<(MatchWithAllInfo, Option<MatchPrediction>)>(&self.connection)?)
    }
}

pub fn bulk_edit(auth: CurrentUser, req: HttpRequest<AppState>) -> impl Responder {
    req.state()
        .db
        .send(FetchBulkPredictionInfo {
            user_id: auth.current_user.user_id,
        })
        .and_then(move |result| match result {
            Ok(matches) => {
                let mut context = Context::new();
                context.add("current_user", &auth.current_user);
                context.add("matches", &matches);

                let rendered = TEMPLATE_SERVICE.render("predictions/bulk_edit.html", &context);

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
        .responder()
}

#[derive(Debug)]
pub struct MatchPredictionItem {
    match_id: i32,
    home_score: i16,
    away_score: i16,
    time_of_first_goal: i16,
}

struct BulkUpdatePredictions {
    user_id: i32,
    match_predictions: Vec<MatchPredictionItem>,
}

impl Message for BulkUpdatePredictions {
    type Result = Result<(), failure::Error>;
}

impl Handler<BulkUpdatePredictions> for DbExecutor {
    type Result = Result<(), failure::Error>;

    fn handle(&mut self, msg: BulkUpdatePredictions, _: &mut Self::Context) -> Self::Result {
        use diesel::{self, insert_into};
        use schema::match_predictions::dsl::*;

        // Update all predictions, or predict none
        self.connection
            .transaction::<_, diesel::result::Error, _>(|| {
                for prediction in msg.match_predictions.iter() {
                    let full_prediction = UpdatedPrediction {
                        user_id: msg.user_id,
                        match_id: prediction.match_id,

                        home_score: prediction.home_score,
                        away_score: prediction.away_score,

                        time_of_first_goal: prediction.time_of_first_goal,
                    };

                    insert_into(match_predictions)
                        .values(&full_prediction)
                        .on_conflict((user_id, match_id))
                        .do_update()
                        .set(&full_prediction)
                        .execute(&self.connection)?;
                }

                Ok(())
            })?;

        Ok(())
    }
}

#[derive(Debug)]
struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Form couldn't be parsed")
    }
}

impl StdError for ParseError {
    fn description(&self) -> &str {
        "Form couldn't be parsed"
    }
}

impl ResponseError for ParseError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::TemporaryRedirect()
            .header("Location", "/")
            .finish()
    }
}

impl FromRequest<AppState> for Vec<MatchPredictionItem> {
    type Config = ();
    type Result = AsyncResult<Self, actix_web::Error>;

    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        let fut = Form::<Vec<(String, String)>>::from_request(
            req,
            &<Form<Vec<(String, String)>> as FromRequest<AppState>>::Config::default(),
        ).and_then(|tuples_form| {
            let tuples = tuples_form.into_inner();
            let mut result = Vec::new();

            fn current_to_prediction(
                val: (String, String, String, String),
            ) -> Result<MatchPredictionItem, failure::Error> {
                Ok(MatchPredictionItem {
                    match_id: val.0.parse()?,
                    home_score: val.1.parse()?,
                    away_score: val.2.parse()?,
                    time_of_first_goal: val.3.parse()?,
                })
            }
            fn process_current(
                result: &mut Vec<MatchPredictionItem>,
                val: (
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                ),
            ) -> Result<(), failure::Error> {
                if val.0 == None || val.1 == None || val.2 == None || val.3 == None {
                    Err(ParseError)?
                } else {
                    if let Ok(prediction) = current_to_prediction((
                        val.0.unwrap(),
                        val.1.unwrap(),
                        val.2.unwrap(),
                        val.3.unwrap(),
                    )) {
                        result.push(prediction);
                    }

                    Ok(())
                }
            }

            if tuples.len() % 4 == 0 {
                let mut current = (None, None, None, None);
                for (key, val) in tuples.iter() {
                    match key.as_str() {
                        "match_id" => current.0 = Some(val.clone()),
                        "home_score" => current.1 = Some(val.clone()),
                        "away_score" => current.2 = Some(val.clone()),
                        "time_of_first_goal" => {
                            current.3 = Some(val.clone());
                            process_current(&mut result, current)?;
                            current = (None, None, None, None)
                        }
                        &_ => Err(ParseError)?,
                    };
                }
                Ok(result)
            } else {
                println!(
                    "There are {} tuples, which is not divisible by 4",
                    tuples.len()
                );
                Err(ParseError)?
            }
        });

        AsyncResult::async(Box::new(fut))
    }
}

pub fn bulk_update(
    auth: CurrentUser,
    form: Vec<MatchPredictionItem>,
    req: HttpRequest<AppState>,
) -> FutureResponse<HttpResponse> {
    req.state()
        .db
        .send(BulkUpdatePredictions {
            user_id: auth.current_user.user_id,
            match_predictions: form,
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
