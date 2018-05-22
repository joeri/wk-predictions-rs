use super::schema::*;
use bcrypt::{hash, DEFAULT_COST};
use chrono::prelude::*;
use diesel;

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize)]
#[primary_key(user_id)]
pub struct User {
    pub user_id: i32,
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
    type Values =
        <NewUserWithEncryptedPassword<'a> as diesel::prelude::Insertable<users::table>>::Values;

    fn values(self) -> Self::Values {
        let plain_text_pw = self.password;
        let hashed_password = match hash(plain_text_pw, DEFAULT_COST) {
            Ok(hashed) => hashed,
            Err(_) => panic!("Error hashing"),
        };

        let encrypted_self = NewUserWithEncryptedPassword {
            email: self.email,
            encrypted_password: hashed_password,
            slack_handle: self.slack_handle,
        };

        encrypted_self.values()
    }
}

#[derive(Queryable, Identifiable, Debug)]
#[primary_key(country_id)]
#[table_name = "countries"]
pub struct Country {
    pub country_id: i32,
    pub name: String,
    pub flag: String,
    pub seeding_pot: String,
}

#[derive(Queryable, Identifiable, Debug)]
#[primary_key(group_id)]
#[table_name = "groups"]
pub struct Group {
    pub group_id: i32,
    pub name: String,
}

#[derive(Queryable, Identifiable, Debug)]
#[primary_key(country_id, group_id)]
#[table_name = "group_memberships"]
pub struct GroupMembership {
    pub country_id: i32,
    pub group_id: i32,
    pub drawn_place: i16,
    pub current_position: i16,
}

#[derive(Queryable, Identifiable, Debug)]
#[primary_key(location_id)]
pub struct Location {
    pub location_id: i32,
    pub city: String,
    pub stadium: String,
    // pub capacity: i32, // not yet
}

#[derive(Queryable, Identifiable, Debug)]
#[primary_key(stage_id)]
pub struct Stage {
    pub stage_id: i32,
    pub parent_stage_id: Option<i32>,
    pub stage_type: StageType,
    pub description: String,
}

#[derive(Queryable, Identifiable, Debug)]
#[primary_key(match_participant_id)]
pub struct MatchParticipant {
    pub match_participant_id: i32,
    pub country_id: Option<i32>,

    // Several fields that determine which type of participant this is,
    // probably should be an option type instead
    pub stage_id: i32,
    pub group_id: Option<i32>,
    pub previous_match_id: Option<i32>,
    pub group_drawn_place: Option<i32>,
    pub result: Option<String>,
}

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize)]
#[primary_key(match_id)]
#[table_name = "matches"]
pub struct Match {
    pub match_id: i32,
    pub stage_id: i32,
    pub location_id: i32,
    pub home_participant_id: i32,
    pub away_participant_id: i32,
    pub time: DateTime<Utc>,
}
