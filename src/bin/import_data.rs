extern crate diesel;
extern crate dotenv;
extern crate csv;
#[macro_use]
extern crate serde_derive;
extern crate chrono;

extern crate wk_predictions;

use diesel::prelude::*;
use diesel::insert_into;
use dotenv::dotenv;

use std::env;
use std::error::Error;
use std::path::Path;

use std::collections::HashMap;

use wk_predictions::{schema, models::*};


#[derive(Deserialize, Debug)]
struct CountryRow {
    name: String,
    flag: String,
    seeding_pot: String,
}

fn import_countries(conn: &PgConnection) -> Result<Vec<Country>, Box<Error>> {
    use schema::countries;

    let mut result = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/countries.csv"))?;
    for row in rdr.deserialize::<CountryRow>() {
        let record = row?;
        println!("{:?}", record);
        let inserted = insert_into(countries::table)
            .values((
                countries::name.eq(record.name),
                countries::flag.eq(record.flag),
                countries::seeding_pot.eq(record.seeding_pot),
            ))
            .returning((countries::country_id, countries::name, countries::flag, countries::seeding_pot))
            .get_result(conn)?;
        println!("{:?}", inserted);
        result.push(inserted);
    }

    Ok(result)
}

#[derive(Deserialize, Debug)]
struct GroupRow {
    id: u64,
    name: String,
}

fn import_groups(conn: &PgConnection) -> Result<Vec<Group>, Box<Error>> {
    use schema::groups;

    let mut result = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/groups.csv"))?;
    for row in rdr.deserialize::<GroupRow>() {
        let record = row?;
        println!("{:?}", record);
        let inserted = insert_into(groups::table)
            .values((
                groups::name.eq(record.name),
            ))
            .returning((groups::group_id, groups::name))
            .get_result(conn)?;
        println!("{:?}", inserted);
        result.push(inserted);
    }

    Ok(result)
}

#[derive(Deserialize, Debug)]
struct GroupMembershipRow {
    country: String,
    group: String,
    drawn_place: String,
}

fn import_group_memberships(conn: &PgConnection) -> Result<Vec<GroupMembership>, Box<Error>> {
    use schema::{group_memberships, groups, countries};
    use diesel::dsl::sql;

    let mut result = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/group_memberships.csv"))?;
    for row in rdr.deserialize::<GroupMembershipRow>() {
        let record = row?;
        println!("{:?}", record);
        let new_groups = groups::table.filter(groups::name.eq(record.group)).inner_join(countries::table.on(countries::name.eq(record.country)));

        let inserted = insert_into(group_memberships::table)
            .values(new_groups.select((groups::group_id, countries::country_id, sql(&record.drawn_place), sql(&record.drawn_place))))
            .into_columns((group_memberships::group_id, group_memberships::country_id, group_memberships::drawn_place, group_memberships::current_position))
            .returning((group_memberships::country_id, group_memberships::group_id, group_memberships::drawn_place, group_memberships::current_position))
            .get_result(conn)?;
        println!("{:?}", inserted);
        result.push(inserted);
    }

    Ok(result)
}

#[derive(Deserialize, Debug)]
struct LocationRow {
    city: String,
    stadium: String,
    capacity: i32,
}

fn import_locations(conn: &PgConnection) -> Result<Vec<Location>, Box<Error>> {
    use schema::locations;

    let mut result = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/locations.csv"))?;
    for row in rdr.deserialize::<LocationRow>() {
        let record = row?;
        println!("{:?}", record);

        let inserted = insert_into(locations::table)
            .values((
                locations::city.eq(record.city),
                locations::stadium.eq(record.stadium),
            ))
            .returning((locations::location_id, locations::city, locations::stadium))
            .get_result(conn)?;
        println!("{:?}", inserted);
        result.push(inserted);
    }

    Ok(result)
}

#[derive(Deserialize, Debug)]
struct StageRow {
    stage_id: i32,
    parent_stage_id: Option<i32>,
    stage_type: String,
    description: String,
}

#[derive(Debug)]
struct StageTypeParseError;

impl std::fmt::Display for StageTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "StageTypeParseError")
    }
}

impl Error for StageTypeParseError {
    fn description(&self) -> &str {
        "Failed to convert stage type"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

fn import_stages(conn: &PgConnection) -> Result<Vec<Stage>, Box<Error>> {
    use schema::{stages, StageType};

    let mut result = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/stages.csv"))?;
    for row in rdr.deserialize::<StageRow>() {
        let record = row?;
        println!("{:?}", record);

        let stage_type = match record.stage_type.as_str() {
            "group" => StageType::Group,
            "knockout" => StageType::Knockout,
            _ => return Err(Box::new(StageTypeParseError)),
        };
        let inserted = insert_into(stages::table)
            .values((
                stages::stage_id.eq(record.stage_id),
                stages::parent_stage_id.eq(record.parent_stage_id),
                stages::stage_type.eq(stage_type),
                stages::description.eq(record.description),
            ))
            .returning((stages::stage_id, stages::parent_stage_id, stages::stage_type, stages::description))
            .get_result(conn)?;
        println!("{:?}", inserted);
        result.push(inserted);
    }

    Ok(result)
}

#[derive(Deserialize, Debug)]
struct MatchRow {
    match_id: i32,
    time: String,
    home_team: String,
    away_team: String,
    stadium: String,
    city: String,
}

fn import_matches(locations: Vec<Location>, countries: Vec<Country>, conn: &PgConnection) -> Result<(Vec<Match>, Vec<(MatchParticipant, MatchParticipant)>), Box<Error>> {
    use schema::{matches, match_participants, group_memberships};
    use chrono::DateTime;

    let mut locations_by_city_and_stadium = HashMap::with_capacity(locations.len());
    for location in locations {
        locations_by_city_and_stadium.insert((location.city.clone(), location.stadium.clone()), location);
    }

    let mut teams_by_country_name = HashMap::with_capacity(countries.len());
    for country in countries {
        teams_by_country_name.insert(country.name.clone(), country);
    }

    let mut matches = Vec::new();
    let mut participants = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/matches.csv"))?;
    for row in rdr.deserialize::<MatchRow>() {
        let record = row?;
        println!("{:?}", record);

        let (group_id, drawn_place) = group_memberships::table
            .filter(
                group_memberships::country_id.eq(teams_by_country_name.get(&record.home_team).unwrap().country_id)
            )
            .select((group_memberships::group_id, group_memberships::drawn_place))
            .first::<(i32, i16)>(conn)?;

        let inserted_home_participant : MatchParticipant = insert_into(match_participants::table)
            .values((
                match_participants::stage_id.eq(1),
                match_participants::country_id.eq(teams_by_country_name.get(&record.home_team).unwrap().country_id),
                match_participants::group_id.eq(group_id),
                match_participants::group_drawn_place.eq(drawn_place as i32),
            ))
            .returning(match_participants::all_columns)
            .get_result(conn)?;
        println!("{:?}", inserted_home_participant);

        let drawn_place2 = group_memberships::table
            .filter(
                group_memberships::country_id.eq(teams_by_country_name.get(&record.away_team).unwrap().country_id)
            )
            .select(group_memberships::drawn_place)
            .first::<i16>(conn)?;

        let inserted_away_participant : MatchParticipant = insert_into(match_participants::table)
            .values((
                match_participants::stage_id.eq(1),
                match_participants::country_id.eq(teams_by_country_name.get(&record.away_team).unwrap().country_id),
                match_participants::group_id.eq(group_id),
                match_participants::group_drawn_place.eq(drawn_place2 as i32),
            ))
            .returning(match_participants::all_columns)
            .get_result(conn)?;

        println!("{:?}", inserted_away_participant);

        let time = DateTime::parse_from_str(&record.time, "%d %B %Y %R %:z")?;

        let inserted_match = insert_into(matches::table)
            .values((
                matches::stage_id.eq(1),
                matches::time.eq(time),
                matches::location_id.eq(locations_by_city_and_stadium.get(&(record.city, record.stadium)).unwrap().location_id),
                matches::home_participant_id.eq(inserted_home_participant.match_participant_id),
                matches::away_participant_id.eq(inserted_away_participant.match_participant_id),
            ))
            .returning(matches::all_columns)
            .get_result(conn)?;

        println!("{:?}", inserted_match);
        participants.push((inserted_home_participant, inserted_away_participant));
        matches.push(inserted_match);
    }

    Ok((matches, participants))
}

fn main() {
    dotenv().ok();

    let database_url: String = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::remove_var("DATABASE_URL"); // Likely contains username/password

    let db_connection = PgConnection::establish(&database_url).unwrap();

    diesel::delete(schema::match_predictions::table).execute(&db_connection).expect("Clearing the match predictions table failed");
    diesel::delete(schema::group_predictions::table).execute(&db_connection).expect("Clearing the group predictions table failed");
    diesel::delete(schema::favourites::table).execute(&db_connection).expect("Clearing the favourites table failed");

    diesel::delete(schema::match_outcomes::table).execute(&db_connection).expect("Clearing the match outcomes table failed");
    diesel::delete(schema::matches::table).execute(&db_connection).expect("Clearing the matches table failed");
    // Note that in theory we can't clear all matches and match participants in one go, as they
    // depend on each other, in practice this script wont run after I've added match participants
    // that point to actual matches
    // Also I'm probably going to reset the database and add ON DELETE
    diesel::delete(schema::match_participants::table).execute(&db_connection).expect("Clearing the match_participants table failed");

    diesel::delete(schema::group_memberships::table).execute(&db_connection).expect("Clearing the group memberships table failed");
    diesel::delete(schema::groups::table).execute(&db_connection).expect("Clearing the groups table failed");

    diesel::delete(schema::locations::table).execute(&db_connection).expect("Clearing the locations table failed");
    diesel::delete(schema::stages::table).execute(&db_connection).expect("Clearing the stages table failed");
    diesel::delete(schema::countries::table).execute(&db_connection).expect("Clearing the countries table failed");

    let locations = import_locations(&db_connection).expect("Import location table failed");
    import_stages(&db_connection).expect("Import stages table failed");
    let countries = import_countries(&db_connection).expect("Import Countries table failed");
    import_groups(&db_connection).expect("Import Groups table failed");
    import_group_memberships(&db_connection).expect("Import Group Memberships table failed");
    import_matches(locations, countries, &db_connection).expect("Import Group Memberships table failed");
}
