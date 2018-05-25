CREATE TABLE matches (
  match_id INTEGER PRIMARY KEY,

  stage_id integer NOT NULL REFERENCES stages,
  location_id integer NOT NULL REFERENCES locations,

  home_participant_id integer NOT NULL REFERENCES match_participants(match_participant_id),
  away_participant_id integer NOT NULL REFERENCES match_participants(match_participant_id),

  time timestamp (0) with time zone NOT NULL -- only need precision up to seconds
);

