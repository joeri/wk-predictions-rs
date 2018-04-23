extern crate actix;
extern crate actix_web;
extern crate diesel;
extern crate dotenv;
extern crate futures;

extern crate wk_predictions;
use wk_predictions::{schema, web::{auth, app_state, app_state::{ AppState, DbExecutor }}};

use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use actix_web::{server, App, AsyncResponder, Error, FutureResponse, HttpResponse,
                Path, State};
use actix::prelude::*;

use futures::future::Future;

struct CountUsers;

impl Message for CountUsers {
    type Result = Result<i64, Error>;
}

impl Handler<CountUsers> for DbExecutor {
    type Result = Result<i64, Error>;

    fn handle(&mut self, _msg: CountUsers, _: &mut Self::Context) -> Self::Result {
        use schema::users::dsl::*;

        // normal diesel operations
        let result = users
            .select(diesel::dsl::count_star())
            .first(&self.connection)
            .expect("Could not determine number of people, had trouble connecting to DB");

        Ok(result)
    }
}


fn index(info: Path<(String, u32)>, state: State<AppState>) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(CountUsers {})
        .from_err()
        .and_then(move |res| match res {
            Ok(count) => Ok(HttpResponse::Ok().body(format!(
                "There are {} user(s), data = {}, {}",
                count, info.0, info.1
            ))),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn main() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let bind_url = env::var("BIND_URL").unwrap_or("127.0.0.1".to_owned());
    let bind_port = env::var("BIND_PORT").unwrap_or("8080".to_owned());
    let url = format!("{}:{}", bind_url, bind_port);

    let sys = actix::System::new("diesel-example");

    // Start 3 parallel db executors
    let addr = SyncArbiter::start(3, move || {
        app_state::establish_connection(&database_url)
    });

    server::new(move || {
        App::with_state(AppState { db: addr.clone() })
            .resource("/login", |r| {
                r.name("login");
                r.get().with(auth::login);
                r.post().with2(auth::perform_login).0.limit(4096);
            })
            .resource("/register", |r| {
                r.get().with(auth::register);
                r.post().with2(auth::perform_registration).0.limit(4096);
            })
            .resource("/{name}/{id}/index.html", |r| r.get().with2(index))
    }).bind(&url)
        .unwrap()
        .start();

    println!("Listening http server {}", url);
    let _ = sys.run();
}
