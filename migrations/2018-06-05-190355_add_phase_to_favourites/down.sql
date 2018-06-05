-- This file should undo anything in `up.sql`

ALTER TABLE favourites
  DROP COLUMN phase,
  ADD UNIQUE (user_id, country_id);
