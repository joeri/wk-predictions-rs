CREATE TYPE stage_type AS ENUM ('group', 'knockout');

CREATE TABLE stages (
  stage_id SERIAL PRIMARY KEY,
  parent_stage_id INTEGER REFERENCES stages(stage_id),
  stage_type stage_type NOT NULL,
  description VARCHAR not null
);
