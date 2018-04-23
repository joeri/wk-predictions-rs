#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;

extern crate actix;
extern crate actix_web;
extern crate futures;
#[macro_use]
extern crate serde_derive;

pub mod web;
pub mod schema;
pub mod models;
