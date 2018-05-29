SELECT
  matches.match_id as match_id,
  matches.location_id as location_id,
  matches.time as time,

  home_participant.group_id as home_group_id,
  home_participant.group_drawn_place as home_group_drawn_place,
  home_participant.previous_match_id as home_previous_match_id,
  home_participant.result as home_previous_match_result,

  home_country.name as home_country_name,
  home_country.flag as home_country_flag,

  away_participant.group_id as away_group_id,
  away_participant.group_drawn_place as away_group_drawn_place,
  away_participant.previous_match_id as away_previous_match_id,
  away_participant.result as away_previous_match_result,

  away_country.name as away_country_name,
  away_country.flag as away_country_flag
FROM
  matches
  INNER JOIN match_participants as home_participant
    ON matches.home_participant_id = home_participant.match_participant_id
  LEFT OUTER JOIN countries as home_country
    ON home_participant.country_id = home_country.country_id
  INNER JOIN match_participants as away_participant
    ON matches.away_participant_id = away_participant.match_participant_id
  LEFT OUTER JOIN countries as away_country
    ON away_participant.country_id = away_country.country_id
WHERE
    matches.time > $1
ORDER BY matches.time ASC
LIMIT $2
;
