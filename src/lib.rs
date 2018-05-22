#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;
extern crate chrono;

extern crate actix;
extern crate actix_web;
extern crate futures;
#[macro_use]
extern crate serde_derive;

extern crate bcrypt;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tera;

pub mod models;
pub mod schema;
pub mod templates;
pub mod web;
