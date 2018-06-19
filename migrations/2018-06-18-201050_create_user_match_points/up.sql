CREATE TABLE user_match_points (
  user_id INTEGER NOT NULL REFERENCES users,
  match_id INTEGER NOT NULL REFERENCES matches,

  favourites INTEGER NOT NULL,
  prediction INTEGER NOT NULL,
  time_of_first_goal INTEGER NOT NULL,
  total INTEGER NOT NULL,

  PRIMARY KEY (user_id, match_id)
);
