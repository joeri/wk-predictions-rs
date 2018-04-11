CREATE TABLE countries (
  country_id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  flag VARCHAR NOT NULL, -- need a flag icon, could use unicode for now
  seeding_pot CHAR(1) NOT NULL
);

