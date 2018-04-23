use actix_web::{Form, FutureResponse, HttpResponse, State, AsyncResponder};
use futures::future;
use super::app_state::AppState;

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
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

pub fn perform_registration(
    form: Form<RegistrationForm>,
    state: State<AppState>,
) -> FutureResponse<HttpResponse> {
    future::ok(HttpResponse::Ok().body("POST register")).responder()
}
