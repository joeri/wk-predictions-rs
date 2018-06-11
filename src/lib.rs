#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tera;

extern crate actix;
extern crate actix_web;
extern crate bcrypt;
extern crate chrono;
extern crate failure;
extern crate futures;
extern crate rand;

pub mod models;
pub mod schema;
pub mod templates;
pub mod web;
