-- Add a phase identifier for the favourites selection, change the uniqueness constraint on user_id + country_id
ALTER TABLE favourites
  ADD COLUMN phase smallint NOT NULL,
  ADD CHECK (case when phase=0 then 0 < choice AND choice <= 4 when phase=1 then 4 < choice AND choice <= 7 when phase=2 then choice=8 end),
  DROP CONSTRAINT favourites_user_id_country_id_key
  ;

ALTER TABLE favourites
  ADD CONSTRAINT favourites_user_id_country_id_key UNIQUE(user_id, phase, country_id),
  ADD CONSTRAINT favourites_user_id_phase_choice_key UNIQUE(user_id, phase, choice)
  ;
