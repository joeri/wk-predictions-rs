CREATE TABLE match_outcomes (
  match_id integer PRIMARY KEY REFERENCES matches,

  home_score smallint NOT NULL,
  away_score smallint NOT NULL,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  time_of_first_goal smallint NOT NULL
);

