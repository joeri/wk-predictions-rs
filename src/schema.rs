#[derive(Debug, DbEnum)]
pub enum StageType {
    Group,
    Knockout,
}

table! {
    countries (country_id) {
        country_id -> Int4,
        name -> Varchar,
        flag -> Varchar,
        seeding_pot -> Bpchar,
    }
}

table! {
    favourites (user_id, choice) {
        user_id -> Int4,
        country_id -> Nullable<Int4>,
        choice -> Int2,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        phase -> Int2,
    }
}

table! {
    group_memberships (group_id, drawn_place) {
        group_id -> Int4,
        country_id -> Int4,
        drawn_place -> Int2,
        current_position -> Int2,
    }
}

table! {
    group_predictions (group_id, user_id) {
        group_id -> Int4,
        user_id -> Int4,
        winner_id -> Int4,
        runnerup_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    groups (group_id) {
        group_id -> Int4,
        name -> Varchar,
    }
}

table! {
    locations (location_id) {
        location_id -> Int4,
        city -> Varchar,
        stadium -> Varchar,
    }
}

table! {
    match_outcomes (match_id) {
        match_id -> Int4,
        home_score -> Int2,
        away_score -> Int2,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        time_of_first_goal -> Int2,
    }
}

table! {
    match_participants (match_participant_id) {
        match_participant_id -> Int4,
        country_id -> Nullable<Int4>,
        stage_id -> Int4,
        group_id -> Nullable<Int4>,
        previous_match_id -> Nullable<Int4>,
        group_drawn_place -> Nullable<Int4>,
        result -> Nullable<Varchar>,
    }
}

table! {
    match_predictions (match_id, user_id) {
        match_id -> Int4,
        user_id -> Int4,
        home_score -> Int2,
        away_score -> Int2,
        time_of_first_goal -> Int2,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    matches (match_id) {
        match_id -> Int4,
        stage_id -> Int4,
        location_id -> Int4,
        home_participant_id -> Int4,
        away_participant_id -> Int4,
        time -> Timestamptz,
    }
}

table! {
    use diesel::sql_types::{Int4, Nullable, Varchar};
    use super::StageTypeMapping;
    stages (stage_id) {
        stage_id -> Int4,
        parent_stage_id -> Nullable<Int4>,
        stage_type -> StageTypeMapping,
        description -> Varchar,
    }
}

table! {
    users (user_id) {
        user_id -> Int4,
        display_name -> Nullable<Varchar>,
        login -> Varchar,
        email -> Varchar,
        score -> Int4,
        encrypted_password -> Varchar,
        slack_handle -> Nullable<Varchar>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    full_match_infos (match_id) {
        match_id -> Int4,
        location_id -> Int4,
        time -> Timestamptz,

        home_group_id -> Nullable<Int4>,
        home_previous_match_id -> Nullable<Int4>,
        home_group_drawn_place -> Nullable<Int4>,
        home_previous_match_result -> Nullable<Varchar>,

        home_country_name -> Nullable<Varchar>,
        home_country_flag -> Nullable<Varchar>,

        away_group_id -> Nullable<Int4>,
        away_previous_match_id -> Nullable<Int4>,
        away_group_drawn_place -> Nullable<Int4>,
        away_previous_match_result -> Nullable<Varchar>,

        away_country_name -> Nullable<Varchar>,
        away_country_flag -> Nullable<Varchar>,
    }
}

joinable!(favourites -> countries (country_id));
joinable!(favourites -> users (user_id));
joinable!(group_memberships -> countries (country_id));
joinable!(group_memberships -> groups (group_id));
joinable!(group_predictions -> groups (group_id));
joinable!(group_predictions -> users (user_id));
joinable!(match_outcomes -> matches (match_id));
joinable!(match_participants -> countries (country_id));
joinable!(match_participants -> stages (stage_id));
joinable!(match_predictions -> matches (match_id));
joinable!(match_predictions -> users (user_id));
joinable!(matches -> locations (location_id));
joinable!(matches -> stages (stage_id));

allow_tables_to_appear_in_same_query!(
    countries,
    favourites,
    group_memberships,
    group_predictions,
    groups,
    locations,
    match_outcomes,
    match_participants,
    match_predictions,
    matches,
    stages,
    users,
);
