extern crate actix;
extern crate actix_web;
extern crate diesel;
extern crate dotenv;
extern crate env_logger;
extern crate futures;

extern crate wk_predictions;
use wk_predictions::web::{app_state, app_state::AppState, auth, dashboard, favourites,
                          match_predictions};

use dotenv::dotenv;
use std::env;

use actix::prelude::*;
use actix_web::{middleware::{identity::{CookieIdentityPolicy, IdentityService},
                             Logger},
                server,
                App};

fn main() {
    dotenv().ok();

    env_logger::init();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::remove_var("DATABASE_URL"); // Likely contains username/password
    let cookie_secret = env::var("COOKIE_SECRET")
        .expect("COOKIE_SECRET must be set (and be at least 32 bytes long)");
    env::remove_var("COOKIE_SECRET");

    let bind_url = env::var("BIND_URL").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let bind_port = env::var("BIND_PORT").unwrap_or_else(|_| "8080".to_owned());
    let url = format!("{}:{}", bind_url, bind_port);

    let sys = actix::System::new("diesel-example");

    // Start 3 parallel db executors
    let addr = SyncArbiter::start(3, move || app_state::establish_connection(&database_url));

    server::new(move || {
        App::with_state(AppState { db: addr.clone() })
            .middleware(Logger::default())
            .middleware(IdentityService::new(
                CookieIdentityPolicy::new(&cookie_secret.clone().into_bytes())
                    .name("auth-cookie")
                    .secure(false),
            ))
            .handler("/assets", actix_web::fs::StaticFiles::new("assets"))
            .resource("/login", |r| {
                r.name("login");
                r.get().f(|_req| auth::login());
                r.post().with3(auth::perform_login).0.limit(4096);
            })
            .resource("/logout", |r| {
                r.get().with2(auth::perform_logout);
            })
            .resource("/register", |r| {
                r.get().f(|_req| auth::register());
                r.post().with2(auth::perform_registration).0.limit(4096);
            })
            .resource("/", |r| r.get().with2(dashboard::index))
            .resource("/index.html", |r| r.get().with2(dashboard::index))
            .resource("/match/{id}/prediction", |r| {
                r.get().with3(match_predictions::edit);
                r.post().with3(match_predictions::update);
            })
            .resource("/favourites", |r| {
                r.get().with2(favourites::edit);
                r.post().with3(favourites::update);
            })
    }).bind(&url)
        .unwrap()
        .start();

    println!("Listening http server {}", url);
    let _ = sys.run();
}
