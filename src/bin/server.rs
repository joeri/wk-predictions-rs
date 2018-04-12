extern crate actix;
extern crate actix_web;
extern crate futures;
extern crate diesel;
extern crate dotenv;

extern crate wk_predictions;
use wk_predictions::schema;
use wk_predictions::models;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use actix_web::{http, server, App, State, Error, HttpResponse, Path, AsyncResponder, FutureResponse};

use actix::prelude::*;

use futures::future::Future;

struct DbExecutor(PgConnection);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

struct CountUsers;

impl Message for CountUsers {
    type Result = Result<i64, Error>;
}

impl Handler<CountUsers> for DbExecutor {
    type Result = Result<i64, Error>;

    fn handle(&mut self, msg: CountUsers, _: &mut Self::Context) -> Self::Result {
        use schema::users::dsl::*;

        // normal diesel operations
        let result = users
            .select(diesel::dsl::count_star())
            .first(&self.0)
            .expect("Could not determine number of people, had trouble connecting to DB");

        Ok(result)
    }
}

/// This is state where we will store *DbExecutor* address.
struct AppState {
    db: Addr<Syn, DbExecutor>,
}

fn index(info: Path<(String, u32)>, state: State<AppState>) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(CountUsers {})
        .from_err()
        .and_then(move |res| match res {
            Ok(count) => Ok(HttpResponse::Ok().body(format!("There are {} user(s), data = {}, {}", count, info.0, info.1))),
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
        DbExecutor(PgConnection::establish(&database_url).unwrap())
    });

    server::new(move || {
        App::with_state(AppState { db: addr.clone() })
            .resource("/{name}/{id}/index.html", |r| r.method(http::Method::GET).with2(index))
    }).bind(&url)
        .unwrap()
        .start();

    println!("Listening http server {}", url);
    let _ = sys.run();
}
