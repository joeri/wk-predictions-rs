#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;

pub mod schema;
pub mod models;
extern crate dotenv;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

use self::models::{NewUser, User};

pub fn create_user<'a>(conn: &PgConnection, email: &'a str, slack_handle: Option<&'a str>) -> User {
    use schema::users;

    let new_user = NewUser {
        email,
        encrypted_password: "",
        slack_handle,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .get_result(conn)
        .expect("Error saving new post")
}

fn main() {
    use schema::users::dsl::*;

    let connection = establish_connection();
    let results = users
        .filter(email.eq("joeri@xaop.com"))
        .limit(1)
        .load::<User>(&connection)
        .expect("Error loading users");

    println!("Displaying {} users", results.len());
    for user in results {
        println!(
            "{}\n----------\n{}",
            user.email,
            user.slack_handle
                .unwrap_or_else(|| "User has no slack handle".to_owned())
        );
    }

    use std::io::stdin;
    println!("What would you like your email to be?");
    let mut new_email = String::new();
    stdin().read_line(&mut new_email).unwrap();
    let new_email = &new_email[..(new_email.len() - 1)]; // Drop the newline character
    println!("\nWhat is your slack handle?\n");
    let mut new_slack_handle = String::new();
    stdin().read_line(&mut new_slack_handle).unwrap();

    let user = create_user(&connection, new_email, Some(&new_slack_handle));
    println!("\nSaved user {} with id {}", user.email, user.id);
}
