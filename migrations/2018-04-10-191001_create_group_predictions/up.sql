CREATE TABLE group_predictions (
  group_id INTEGER NOT NULL REFERENCES groups,
  user_id INTEGER NOT NULL REFERENCES users,

  winner_id integer NOT NULL REFERENCES countries(country_id),
  runnerup_id integer NOT NULL REFERENCES countries(country_id),

  -- not valid constraints unless we add a unique constraint on group_memberships
  -- FOREIGN KEY (group_id, winner_id) REFERENCES group_memberships (group_id, country_id)
  -- FOREIGN KEY (group_id, runnerup_id) REFERENCES group_memberships (group_id, country_id)

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  PRIMARY KEY (group_id, user_id)
);

