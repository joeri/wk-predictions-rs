CREATE TABLE match_participants (
  match_participant_id SERIAL PRIMARY KEY,

  country_id integer REFERENCES countries -- Once the result is known
);
