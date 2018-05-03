extern crate diesel;
extern crate dotenv;

extern crate wk_predictions;
use wk_predictions::schema;
use wk_predictions::models;

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

pub fn create_user<'a>(conn: &PgConnection, email: &'a str, password: &'a str, slack_handle: Option<&'a str>) -> User {
    use schema::users;

    let new_user = NewUser {
        email,
        password,
        slack_handle,
    };

    diesel::insert_into(users::table)
        .values(new_user)
        .get_result(conn)
        .expect("Error saving new user")
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
    let new_email_str = &new_email[..(new_email.len() - 1)]; // Drop the newline character
    println!("\npassword: ");
    let mut password = String::new();
    stdin().read_line(&mut password).unwrap();
    let password_str = &password[..(password.len() - 1)]; // Drop the newline character
    println!("\nWhat is your slack handle?\n");
    let mut slack_handle_buf = String::new();
    stdin().read_line(&mut slack_handle_buf).unwrap();
    let new_slack_handle: Option<&str> = if slack_handle_buf == "\n" {
        None
    } else {
        Some(&slack_handle_buf)
    };

    let user = create_user(&connection, new_email_str, password_str, new_slack_handle);
    println!("\nSaved user {} with id {}", user.email, user.id);
}
