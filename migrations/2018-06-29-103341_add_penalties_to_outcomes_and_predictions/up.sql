ALTER TABLE match_outcomes
  ADD COLUMN home_penalties integer,
  ADD COLUMN away_penalties integer,
  ADD COLUMN duration integer;

ALTER TABLE match_predictions
  ADD COLUMN home_penalties integer,
  ADD COLUMN away_penalties integer,
  ADD COLUMN duration integer;
