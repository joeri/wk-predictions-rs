CREATE TABLE match_predictions (
  match_id integer REFERENCES matches,
  user_id integer REFERENCES users,

  home_score smallint NOT NULL,
  away_score smallint NOT NULL,

  time_of_first_goal smallint NOT NULL,

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (match_id, user_id)
);
