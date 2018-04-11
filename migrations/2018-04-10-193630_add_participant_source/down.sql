ALTER TABLE match_participants
  DROP COLUMN stage_id,

  DROP COLUMN group_id, -- for group stage and first knockout round
  DROP COLUMN previous_match_id, -- all other knockout rounds

  DROP COLUMN group_drawn_place, -- for games in group stage
  DROP COLUMN result; -- winner, runnerup or loser (for games in knockout stage)
