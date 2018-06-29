ALTER TABLE match_outcomes
  DROP COLUMN home_penalties,
  DROP COLUMN away_penalties,
  DROP COLUMN duration;

ALTER TABLE match_predictions
  DROP COLUMN home_penalties,
  DROP COLUMN away_penalties,
  DROP COLUMN duration;
