-- TODO: think if this can't be done better with some other kind of serialization
CREATE TABLE favourites (
  user_id INTEGER NOT NULL REFERENCES users,
  country_id INTEGER REFERENCES countries,
  choice smallint CHECK (0 < choice AND choice <= 8),

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

  UNIQUE (user_id, country_id),
  PRIMARY KEY (user_id, choice)
);
