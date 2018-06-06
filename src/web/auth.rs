use actix::prelude::*;
use actix_web::{self, dev::AsyncResult, error::ResponseError,
                middleware::identity::RequestIdentity, AsyncResponder, Form, FromRequest,
                FutureResponse, HttpRequest, HttpResponse, State};
use bcrypt::verify;
use diesel::{self, prelude::*};
use failure;
use futures::{future, Future};
use models::{NewUser, User};
use std::error::Error as StdError;
use std::fmt;
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::{AppState, DbExecutor};

pub struct CurrentUser {
    pub current_user: User,
}

impl FromRequest<AppState> for CurrentUser {
    type Config = ();
    type Result = AsyncResult<Self, actix_web::Error>;

    fn from_request(req: &HttpRequest<AppState>, _cfg: &Self::Config) -> Self::Result {
        match req.identity() {
            Some(current_user_id_string) => match current_user_id_string.parse() {
                Ok(current_user_id) => AsyncResult::async(Box::new(
                    req.state()
                        .db
                        .send(FetchCurrentUser {
                            user_id: current_user_id,
                        })
                        .then(|x| match x {
                            Ok(Ok(x)) => future::ok(CurrentUser { current_user: x }),
                            Ok(Err(y)) => future::err(y.into()),
                            Err(_) => future::err(Unauthenticated.into()),
                        }),
                )),
                _ => AsyncResult::err(Unauthenticated),
            },
            None => AsyncResult::err(Unauthenticated),
        }
    }
}

#[derive(Debug)]
pub struct Unauthenticated;

impl fmt::Display for Unauthenticated {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "User not authenticated")
    }
}

impl StdError for Unauthenticated {
    fn description(&self) -> &str {
        "User not authenticated"
    }
}

impl ResponseError for Unauthenticated {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::TemporaryRedirect()
            .header("Location", "/login")
            .finish()
    }
}

#[derive(Debug)]
pub struct UserNotFoundError;

impl fmt::Display for UserNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "User not found")
    }
}

impl StdError for UserNotFoundError {
    fn description(&self) -> &str {
        "User not found"
    }
}

#[derive(Deserialize, Debug, Clone)]
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

pub fn login() -> HttpResponse {
    let rendered = TEMPLATE_SERVICE.render("login.html", &Context::new());
    match rendered {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(error) => {
            println!("{}", error);
            HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("Something went wrong")
        }
    }
}

pub struct FetchCurrentUser {
    user_id: i32,
}

impl Message for FetchCurrentUser {
    type Result = Result<User, failure::Error>;
}

impl Handler<FetchCurrentUser> for DbExecutor {
    type Result = Result<User, failure::Error>;

    fn handle(&mut self, msg: FetchCurrentUser, _: &mut Self::Context) -> Self::Result {
        use schema::users::dsl::*;

        // normal diesel operations
        let result = users
            .filter(user_id.eq(msg.user_id))
            .first(&self.connection)
            .map_err(|_| Unauthenticated)?;

        Ok(result)
    }
}

impl Message for LoginForm {
    type Result = Result<i32, failure::Error>;
}

impl Handler<LoginForm> for DbExecutor {
    type Result = Result<i32, failure::Error>;

    fn handle(&mut self, form: LoginForm, _context: &mut Self::Context) -> Self::Result {
        use schema::users;

        let user = users::table
            .filter(users::email.eq(form.username))
            .first::<User>(&self.connection)
            .optional()?;

        match user {
            Some(u) => {
                verify(&form.password, &u.encrypted_password)?;
                Ok(u.user_id)
            }
            None => Err(UserNotFoundError)?,
        }
    }
}

pub fn perform_login(
    form: Form<LoginForm>,
    state: State<AppState>,
    mut req: HttpRequest<AppState>,
) -> FutureResponse<HttpResponse> {
    let inner_form = form.into_inner();
    state
        .db
        .send(inner_form.clone())
        .from_err()
        .and_then(move |res| match res {
            Ok(user_id) => {
                req.remember(user_id.to_string());
                // TODO: check whether user was redirected to login, and redirect there instead
                Ok(HttpResponse::SeeOther().header("Location", "/").finish())
            }
            Err(_) => Ok(HttpResponse::InternalServerError()
                .content_type("text/plain; charset=utf-8")
                .body("An unexpected error occurred")),
        })
        .responder()
}

pub fn register() -> HttpResponse {
    let rendered = TEMPLATE_SERVICE.render("register.html", &Context::new());
    match rendered {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(error) => {
            println!("{}", error);
            HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("Something went wrong")
        }
    }
}

impl Message for RegistrationForm {
    type Result = Result<User, diesel::result::Error>;
}
impl Handler<RegistrationForm> for DbExecutor {
    type Result = Result<User, diesel::result::Error>;

    fn handle(&mut self, form: RegistrationForm, _: &mut Self::Context) -> Self::Result {
        use schema::users;

        // TODO: add minimumum length on password
        // Possibly that should be in NewUser, however the values function doesn't accept
        // validation, so should be different step, perhaps NewUser constructor
        let user = NewUser {
            email: &form.username,
            login: &form.username,
            display_name: Some(&form.name),
            password: &form.password,
            slack_handle: None,
        };

        diesel::insert_into(users::table)
            .values(user)
            .get_result(&self.connection)
    }
}

pub fn perform_registration(
    form: Form<RegistrationForm>,
    state: State<AppState>,
) -> FutureResponse<HttpResponse> {
    use diesel::result::{DatabaseErrorKind, Error::DatabaseError};

    let inner_form = form.into_inner();
    state
        .db
        .send(inner_form.clone())
        .from_err()
        .and_then(move |res| match res {
            Ok(user) => /* should be redirect to login really */ Ok(HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(format!(
                "Successfully registered {} as {} with id {}",
                inner_form.username, user.email, user.user_id
            ))),
            Err(DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => Ok(HttpResponse::PreconditionFailed().content_type("text/plain; charset=utf-8").body(format!("User {} already registered", inner_form.username))),
            Err(_) => Ok(HttpResponse::InternalServerError().content_type("text/plain; charset=utf-8").body("An unexpected error occurred")),
        })
        .responder()
}

pub fn perform_logout(current_user: CurrentUser, mut req: HttpRequest<AppState>) -> HttpResponse {
    print!("Logging out user {:?}", current_user.current_user);
    req.forget();

    HttpResponse::TemporaryRedirect()
        .header("Location", "/login")
        .finish()
}
