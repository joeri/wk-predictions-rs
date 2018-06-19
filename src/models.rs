use super::schema::*;
use bcrypt::{hash, DEFAULT_COST};
use chrono::prelude::*;
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Clone, Debug, Serialize, Deserialize)]
#[primary_key(user_id)]
pub struct User {
    pub user_id: i32,
    pub display_name: Option<String>,
    pub login: String,
    pub email: String,
    pub score: i32,
    pub encrypted_password: String,
    pub slack_handle: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct NewUser<'a> {
    pub email: &'a str,
    pub login: &'a str,
    pub display_name: Option<&'a str>,
    pub password: &'a str,
    pub slack_handle: Option<&'a str>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUserWithEncryptedPassword<'a> {
    pub email: &'a str,
    pub login: &'a str,
    pub display_name: Option<&'a str>,
    pub encrypted_password: String,
    pub slack_handle: Option<&'a str>,
}

impl<'a> Insertable<users::table> for NewUser<'a> {
    type Values = <NewUserWithEncryptedPassword<'a> as Insertable<users::table>>::Values;

    fn values(self) -> Self::Values {
        let plain_text_pw = self.password;
        let hashed_password = match hash(plain_text_pw, DEFAULT_COST) {
            Ok(hashed) => hashed,
            Err(_) => panic!("Error hashing"),
        };

        let encrypted_self = NewUserWithEncryptedPassword {
            email: self.email,
            login: self.login,
            display_name: self.display_name,
            encrypted_password: hashed_password,
            slack_handle: self.slack_handle,
        };

        encrypted_self.values()
    }
}

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize)]
#[primary_key(country_id)]
#[table_name = "countries"]
pub struct Country {
    pub country_id: i32,
    pub name: String,
    pub flag: String,
    pub seeding_pot: String,
}

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize)]
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

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize)]
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

#[derive(Queryable, Identifiable, Debug, Serialize, Deserialize, Clone)]
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

pub struct MatchWithParticipants {
    pub match_id: i32,
    pub home_participant: MatchParticipant,
    pub away_participant: MatchParticipant,
    pub time: DateTime<Utc>,
}

#[derive(Queryable, Identifiable, Associations, Debug, Serialize, Deserialize, Clone)]
#[belongs_to(User)]
#[primary_key(match_id, user_id)]
pub struct MatchPrediction {
    pub match_id: i32,
    pub user_id: i32,

    pub home_score: i16,
    pub away_score: i16,
    pub time_of_first_goal: i16,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    pub source: String,
}

#[derive(Debug, Insertable, AsChangeset)]
#[table_name = "match_predictions"]
pub struct UpdatedPrediction {
    pub match_id: i32,
    pub user_id: i32,

    pub home_score: i16,
    pub away_score: i16,

    pub time_of_first_goal: i16,
}

#[derive(Debug, Insertable, AsChangeset)]
#[table_name = "match_predictions"]
pub struct PredictionWithSource {
    pub match_id: i32,
    pub user_id: i32,

    pub home_score: i16,
    pub away_score: i16,

    pub time_of_first_goal: i16,

    pub source: String,
}

#[derive(Queryable, Identifiable, Insertable, Debug, Serialize, Deserialize, AsChangeset, Clone)]
#[primary_key(match_id)]
pub struct MatchOutcome {
    pub match_id: i32,

    pub home_score: i16,
    pub away_score: i16,
    pub time_of_first_goal: i16,
}

// I should consider adding a view according to this data
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, QueryableByName)]
#[table_name = "full_match_infos"]
pub struct MatchWithAllInfo {
    pub match_id: i32,
    pub location_id: i32,
    pub time: DateTime<Utc>,

    pub home_group_id: Option<i32>,
    pub home_group_drawn_place: Option<i32>,
    pub home_previous_match_id: Option<i32>,
    pub home_previous_match_result: Option<String>,

    pub home_country_name: Option<String>,
    pub home_country_flag: Option<String>,

    pub away_group_id: Option<i32>,
    pub away_group_drawn_place: Option<i32>,
    pub away_previous_match_id: Option<i32>,
    pub away_previous_match_result: Option<String>,

    pub away_country_name: Option<String>,
    pub away_country_flag: Option<String>,
}

#[derive(Debug, Clone, Associations, Serialize, Deserialize, Queryable, Identifiable)]
#[belongs_to(User)]
#[primary_key(user_id, choice)]
pub struct Favourite {
    pub user_id: i32,
    pub country_id: Option<i32>,
    pub choice: i16,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub phase: i16,
    pub source: String,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "favourites"]
pub struct UpdatedFavourite {
    pub user_id: i32,
    pub country_id: Option<i32>,
    pub choice: i16,
    pub phase: i16,
}

#[derive(Queryable, Insertable, AsChangeset)]
#[table_name = "user_match_points"]
pub struct UserMatchPoints {
    pub user_id: i32,
    pub match_id: i32,
    pub favourites: i32,
    pub prediction: i32,
    pub time_of_first_goal: i32,
    pub total: i32,
}
