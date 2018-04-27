use super::schema::users;
use bcrypt::{hash, DEFAULT_COST};
use diesel;

#[derive(Queryable, Identifiable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub encrypted_password: String,
    pub slack_handle: Option<String>,
}

pub struct NewUser<'a> {
    pub email: &'a str,
    pub password: &'a str,
    pub slack_handle: Option<&'a str>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUserWithEncryptedPassword<'a> {
    pub email: &'a str,
    pub encrypted_password: String,
    pub slack_handle: Option<&'a str>,
}

impl<'a> diesel::prelude::Insertable<users::table> for NewUser<'a> {
    type Values = <NewUserWithEncryptedPassword<'a> as diesel::prelude::Insertable<users::table>>::Values;

    fn values(self) -> Self::Values {
        let plain_text_pw = self.password;
        let hashed_password = match hash(plain_text_pw, DEFAULT_COST) {
            Ok(hashed) => hashed,
            Err(_) => panic!("Error hashing")
        };

        let encrypted_self = NewUserWithEncryptedPassword {
            email: self.email,
            encrypted_password: hashed_password,
            slack_handle: self.slack_handle,
        };

        encrypted_self.values()
    }
}




