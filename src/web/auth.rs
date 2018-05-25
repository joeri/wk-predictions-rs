use actix::prelude::*;
use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, Form, FutureResponse, HttpRequest, HttpResponse, State};
use bcrypt::verify;
use diesel;
use diesel::prelude::*;
use futures::Future;
use models::{NewUser, User};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::{AppState, DbExecutor};
use failure;

use actix_web::Error;

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
    type Result = Result<User, Error>;
}

impl Handler<FetchCurrentUser> for DbExecutor {
    type Result = Result<User, Error>;

    fn handle(&mut self, msg: FetchCurrentUser, _: &mut Self::Context) -> Self::Result {
        use schema::users::dsl::*;

        // normal diesel operations
        let result = users
            .filter(user_id.eq(msg.user_id))
            .first(&self.connection)
            .expect("Could not extract current user from DB");

        Ok(result)
    }
}

impl Message for LoginForm {
    type Result = Result<bool, failure::Error>;
}

impl Handler<LoginForm> for DbExecutor {
    type Result = Result<bool, failure::Error>;

    fn handle(&mut self, form: LoginForm, _context: &mut Self::Context) -> Self::Result {
        use schema::users;

        let user = users::table
            .filter(users::email.eq(form.username))
            .first::<User>(&self.connection)
            .optional()?;

        let result = match user {
            Some(u) => verify(&form.password, &u.encrypted_password)?,
            None => false,
        };

        // TODO: set cookie (perhaps in perform_login method itself)
        // TODO: provide better error (perhaps in result, but probably need to have better error
        // return mechanism)

        Ok(result)
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
            Ok(true) => {
                req.remember(inner_form.username.clone());
                // TODO: check whether user was redirected to login, and redirect there instead
                Ok(HttpResponse::SeeOther().header("Location", "/").finish())
            },
            Ok(false) => /* should be redirect to /login really */ Ok(HttpResponse::Unauthorized().content_type("text/plain; charset=utf-8").body(format!(
                "Failed to login by {:?}",
                inner_form.username,
            ))),
            Err(_) => Ok(HttpResponse::InternalServerError().content_type("text/plain; charset=utf-8").body("An unexpected error occurred")),
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
