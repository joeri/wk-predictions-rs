use models::{
    Favourite, MatchOutcome, MatchPrediction, MatchWithParticipants, User, UserMatchPoints,
};

use std::cmp::max;

fn phase_of(game: &MatchWithParticipants) -> i16 {
    match game.stage_id {
        1 => 0,
        2 | 3 => 1,
        4 | 5 | 6 => 2,
        _ => unreachable!(),
    }
}

fn favourite_points(
    favourite: &Favourite,
    game: &MatchWithParticipants,
    outcome: &MatchOutcome,
) -> i16 {
    if favourite.updated_at >= game.time.naive_utc() {
        return 0;
    }
    if favourite.phase != phase_of(game) {
        return 0;
    }

    if favourite.country_id == game.home_participant.country_id {
        outcome.home_score + if outcome.home_score > outcome.away_score {
            3
        } else if outcome.home_score == outcome.away_score {
            1
        } else {
            0
        }
    } else if favourite.country_id == game.away_participant.country_id {
        outcome.away_score + if outcome.away_score > outcome.home_score {
            3
        } else if outcome.home_score == outcome.away_score {
            1
        } else {
            0
        }
    } else {
        0
    }
}

fn prediction_and_tofg_points(
    prediction: &MatchPrediction,
    game: &MatchWithParticipants,
    outcome: &MatchOutcome,
) -> (i32, i32) {
    if prediction.updated_at >= game.time.naive_utc() {
        return (0, 0);
    }

    fn winner(a: i16, b: i16) -> i8 {
        if a > b {
            1
        } else if a == b {
            0
        } else {
            -1
        }
    }

    let predicted_winner = winner(prediction.home_score, prediction.away_score);
    let actual_winner = winner(outcome.home_score, outcome.away_score);

    let mut result = 0;
    if predicted_winner == actual_winner {
        result += 2;
    }
    if prediction.home_score == outcome.home_score {
        result += 1;
    }
    if prediction.away_score == outcome.away_score {
        result += 1;
    }
    if result == 4 {
        // Outcome equals prediction completely
        result += 3;
    }

    let points_for_time_of_goal = if predicted_winner == actual_winner {
        max(
            0,
            5 - (outcome.time_of_first_goal - prediction.time_of_first_goal).abs(),
        )
    } else {
        0
    };

    (result, points_for_time_of_goal.into())
}

pub fn user_match_points(
    user_with_prediction: &(User, Option<MatchPrediction>, Vec<Favourite>),
    game: &(MatchWithParticipants, MatchOutcome),
) -> UserMatchPoints {
    let mut fav_points = 0;
    for favourite in &user_with_prediction.2 {
        fav_points += i32::from(favourite_points(&favourite, &game.0, &game.1));
    }
    let (prediction_points, tofg_points) = if let Some(prediction) = &user_with_prediction.1 {
        prediction_and_tofg_points(&prediction, &game.0, &game.1)
    } else {
        (0, 0)
    };

    UserMatchPoints {
        user_id: user_with_prediction.0.user_id,
        match_id: game.0.match_id,

        favourites: fav_points,
        prediction: prediction_points,
        time_of_first_goal: tofg_points,

        total: prediction_points + tofg_points + fav_points,
    }
}
