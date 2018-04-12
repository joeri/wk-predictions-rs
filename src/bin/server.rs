extern crate diesel;
extern crate actix_web;
extern crate dotenv;

extern crate wk_predictions;
use wk_predictions::schema;
use wk_predictions::models;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

use actix_web::{server, App, Path};

fn index(info: Path<(String, u32)>) -> String {
   format!("Hello {}! id:{}", info.0, info.1)
}

fn main() {
    dotenv().ok();

    let bind_url = env::var("BIND_URL").unwrap_or("127.0.0.1".to_owned());
    let bind_port = env::var("BIND_PORT").unwrap_or("8080".to_owned());
    let url = format!("{}:{}", bind_url, bind_port);
    println!("Listening on {}, not on {}", url, "127.0.0.1:8080");

    server::new(
        || App::new()
            .resource("/{name}/{id}/index.html", |r| r.with(index)))
        .bind(url).unwrap()
        .run();
}
