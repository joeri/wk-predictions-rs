use actix_web;
use actix_web::{Form, FutureResponse, HttpResponse, State, AsyncResponder, Error};
use actix::prelude::*;
use futures::{future, Future};
use diesel;
use diesel::prelude::*;
use ::web::app_state::{AppState, DbExecutor};
use ::models::{User, NewUser};

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize, Clone)]
pub struct RegistrationForm {
    username: String,
    name: String,
    password: String,
}

pub fn login(state: State<AppState>) -> HttpResponse {
    HttpResponse::Ok().body("login")
}

pub fn perform_login(form: Form<LoginForm>, state: State<AppState>) -> FutureResponse<HttpResponse> {
    future::ok(HttpResponse::Ok().body("POST login")).responder()
}

pub fn register(state: State<AppState>) -> FutureResponse<HttpResponse> {
    future::ok(HttpResponse::Ok().body("GET register")).responder()
}

impl Message for RegistrationForm {
    type Result = Result<User, Error>;
}
impl Handler<RegistrationForm> for DbExecutor {
    type Result = Result<User, Error>;

    fn handle(&mut self, form: RegistrationForm, _: &mut Self::Context) -> Self::Result {
        use schema::users;

        let user = NewUser {
            email: &form.username,
            password: &form.password,
            slack_handle: None,
        };

        diesel::insert_into(users::table)
            .values(user)
            .get_result(&self.connection)
            .map_err(actix_web::error::ErrorBadRequest)
    }
}

pub fn perform_registration(
    form: Form<RegistrationForm>,
    state: State<AppState>,
) -> FutureResponse<HttpResponse> {
    let inner_form = form.into_inner();
    state
        .db
        .send(inner_form.clone())
        .from_err()
        .and_then(move |res| match res {
            Ok(user) => Ok(HttpResponse::Ok().body(format!(
                "Successfully registered {} as {} with id {}",
                inner_form.username, user.email, user.id
            ))),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}
