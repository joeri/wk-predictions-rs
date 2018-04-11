ALTER TABLE match_participants
  ADD COLUMN stage_id integer NOT NULL references stages,

  ADD COLUMN group_id integer, -- for group stage and first knockout round
  ADD COLUMN previous_match_id integer references matches(match_id), -- all other knockout rounds

  ADD COLUMN group_drawn_place integer, -- for games in group stage
  ADD COLUMN result VARCHAR; -- winner, runnerup or loser (for games in knockout stage)
