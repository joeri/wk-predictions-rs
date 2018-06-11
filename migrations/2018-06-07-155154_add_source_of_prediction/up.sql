ALTER TABLE match_predictions
  ADD COLUMN source varchar not null default 'manual';

ALTER TABLE favourites
  ADD COLUMN source varchar not null default 'manual';
