extern crate csv;
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate failure;

extern crate wk_predictions;

use diesel::insert_into;
use diesel::prelude::*;
use dotenv::dotenv;

use std::env;
use std::error::Error;
use std::path::Path;

use std::collections::HashMap;

use wk_predictions::{models::*, schema};

#[derive(Deserialize, Debug)]
struct MatchRow {
    stage_id: i32,
    match_id: i32,
    time: String,
    home_country: String,
    home_condition: String,
    home_source: String,
    away_country: String,
    away_condition: String,
    away_source: String,
    stadium: String,
}

fn insert_participant(
    stage: i32,
    team: String,
    condition: String,
    source: String,
    teams_by_country_name: &HashMap<String, Country>,
    groups_by_name: &HashMap<String, Group>,
    conn: &PgConnection,
) -> Result<MatchParticipant, failure::Error> {
    let country = if stage == 2 {
        Some(teams_by_country_name[&team].country_id)
    } else {
        None
    };
    let (previous_match, group) = match source.parse::<i32>() {
        Ok(match_id) => (Some(match_id), None),
        Err(_) => (None, Some(groups_by_name[&source].group_id)),
    };

    let inserted_participant: MatchParticipant = {
        use schema::match_participants::{all_columns, dsl::*};

        insert_into(match_participants)
            .values((
                stage_id.eq(stage),
                country_id.eq(country),
                group_id.eq(group),
                previous_match_id.eq(previous_match),
                result.eq(condition),
            ))
            .returning(all_columns)
            .get_result(conn)?
    };
    Ok(inserted_participant)
}

fn import_matches(
    locations: Vec<Location>,
    countries: Vec<Country>,
    groups: Vec<Group>,
    conn: &PgConnection,
) -> Result<(Vec<Match>, Vec<(MatchParticipant, MatchParticipant)>), failure::Error> {
    use chrono::DateTime;
    use schema::matches;

    let mut locations_by_stadium = HashMap::with_capacity(locations.len());
    for location in locations {
        locations_by_stadium.insert(location.stadium.clone(), location);
    }

    let mut teams_by_country_name = HashMap::with_capacity(countries.len());
    for country in countries {
        teams_by_country_name.insert(country.name.clone(), country);
    }

    let mut groups_by_name = HashMap::with_capacity(groups.len());
    for group in groups {
        groups_by_name.insert(group.name.clone(), group);
    }

    let mut participants = Vec::new();
    let mut matches = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/knockout-phase.csv"))?;
    for row in rdr.deserialize::<MatchRow>() {
        println!("{:?}", row);
        let record = row?;

        let home_participant = insert_participant(
            record.stage_id,
            record.home_country,
            record.home_condition,
            record.home_source,
            &teams_by_country_name,
            &groups_by_name,
            conn,
        )?;
        let away_participant = insert_participant(
            record.stage_id,
            record.away_country,
            record.away_condition,
            record.away_source,
            &teams_by_country_name,
            &groups_by_name,
            conn,
        )?;

        let time = DateTime::parse_from_str(&record.time, "%d/%m/%Y %H:%M%z")?;
        println!("Managed to parse date");
        let location_id = locations_by_stadium[&record.stadium].location_id;
        println!("Managed to find location");

        let inserted_match = insert_into(matches::table)
            .values((
                matches::match_id.eq(record.match_id),
                matches::stage_id.eq(record.stage_id),
                matches::time.eq(time),
                matches::location_id.eq(location_id),
                matches::home_participant_id.eq(home_participant.match_participant_id),
                matches::away_participant_id.eq(away_participant.match_participant_id),
            ))
            .returning(matches::all_columns)
            .get_result(conn)?;

        println!("{:?}", inserted_match);
        matches.push(inserted_match);
        participants.push((home_participant.clone(), away_participant.clone()))
    }

    Ok((matches, participants))
}

fn do_import(db_connection: &PgConnection) -> Result<(), failure::Error> {
    let locations = schema::locations::table.load(db_connection)?;
    let countries = schema::countries::table.load(db_connection)?;
    let groups = schema::groups::table.load(db_connection)?;

    db_connection.transaction(|| import_matches(locations, countries, groups, db_connection))?;
    Ok(())
}

fn main() {
    dotenv().ok();

    let database_url: String = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::remove_var("DATABASE_URL"); // Likely contains username/password

    let db_connection = PgConnection::establish(&database_url).unwrap();

    do_import(&db_connection);
}
