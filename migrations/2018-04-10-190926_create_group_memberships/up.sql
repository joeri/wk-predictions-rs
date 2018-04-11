CREATE TABLE group_memberships (
  group_id integer NOT NULL REFERENCES groups(group_id),
  country_id integer NOT NULL REFERENCES countries(country_id),
  drawn_place smallint NOT NULL,
  current_position smallint NOT NULL,

  PRIMARY KEY (group_id, drawn_place),
  UNIQUE (country_id)
);
