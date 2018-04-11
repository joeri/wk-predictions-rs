-- Your SQL goes here
CREATE TABLE users (
  user_id SERIAL PRIMARY KEY,
  email VARCHAR NOT NULL,
  encrypted_password VARCHAR NOT NULL,
  slack_handle VARCHAR
);

CREATE UNIQUE INDEX unique_users_email ON users (email)
