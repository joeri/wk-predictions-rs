extern crate diesel;
extern crate dotenv;
extern crate csv;
#[macro_use]
extern crate serde_derive;

extern crate wk_predictions;

use diesel::prelude::*;
use diesel::insert_into;
use dotenv::dotenv;

use std::env;
use std::error::Error;
use std::path::Path;

use wk_predictions::{schema, models::{Country, Group, GroupMembership}};

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
    drawn_place: i16,
}

fn import_group_memberships(conn: &PgConnection) -> Result<Vec<GroupMembership>, Box<Error>> {
    use schema::{group_memberships, groups, countries};

    let mut result = Vec::new();

    let mut rdr = csv::Reader::from_path(Path::new("data/group_memberships.csv"))?;
    for row in rdr.deserialize::<GroupMembershipRow>() {
        let record = row?;
        println!("{:?}", record);
        let country_and_group = groups::table.filter(groups::name.eq(record.group)).inner_join(countries::table.on(countries::name.eq(record.country))).select((countries::country_id, groups::group_id)).first::<(i32,i32)>(conn)?;

        let inserted = insert_into(group_memberships::table)
            .values((
                group_memberships::country_id.eq(country_and_group.0),
                group_memberships::group_id.eq(country_and_group.1),
                group_memberships::drawn_place.eq(record.drawn_place),
                group_memberships::current_position.eq(record.drawn_place),
            ))
            .returning((group_memberships::country_id, group_memberships::group_id, group_memberships::drawn_place, group_memberships::current_position))
            .get_result(conn)?;
        println!("{:?}", inserted);
        result.push(inserted);
    }

    Ok(result)
}

fn main() {
    dotenv().ok();

    let database_url: String = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    env::remove_var("DATABASE_URL"); // Likely contains username/password

    let db_connection = PgConnection::establish(&database_url).unwrap();

    diesel::delete(schema::group_memberships::table).execute(&db_connection).expect("Clearing the group memberships table failed");
    diesel::delete(schema::groups::table).execute(&db_connection).expect("Clearing the groups table failed");
    diesel::delete(schema::countries::table).execute(&db_connection).expect("Clearing the countries table failed");

    import_countries(&db_connection).expect("Import Countries table failed");
    import_groups(&db_connection).expect("Import Groups table failed");
    import_group_memberships(&db_connection).expect("Import Group Memberships table failed");
}
