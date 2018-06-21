use models::User;
use templates::{Context, TEMPLATE_SERVICE};
use web::{
    app_state::{AppState, DbExecutor}, auth::CurrentUser,
};

use actix::prelude::*;
use actix_web::{
    AsyncResponder, FutureResponse, HttpRequest, HttpResponse, Query, Responder, State,
};
use futures::Future;

use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use failure;

mod user_points {
    use diesel::sql_types::*;

    #[derive(QueryableByName, Serialize, Deserialize)]
    pub struct UserPoints {
        #[sql_type = "Text"]
        pub display_name: String,
        #[sql_type = "BigInt"]
        pub prediction: i64,
        #[sql_type = "BigInt"]
        pub favourites: i64,
        #[sql_type = "BigInt"]
        pub time_of_first_goal: i64,
        #[sql_type = "BigInt"]
        pub score: i64,
    }
}

#[derive(Deserialize, Clone)]
pub struct FetchLeaderBoard {
    up_to: Option<i64>,
}

impl Message for FetchLeaderBoard {
    type Result = Result<(Vec<user_points::UserPoints>, Option<DateTime<Utc>>), failure::Error>;
}

impl Handler<FetchLeaderBoard> for DbExecutor {
    type Result = Result<(Vec<user_points::UserPoints>, Option<DateTime<Utc>>), failure::Error>;

    fn handle(&mut self, msg: FetchLeaderBoard, _: &mut Self::Context) -> Self::Result {
        use diesel::sql_query;
        use diesel::sql_types::Nullable;
        use diesel::sql_types::Timestamptz;

        if let Some(up_to) = msg.up_to {
            let up_to_chrono =
                DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(up_to, 0), Utc);
            let leaders = sql_query(
                "
            SELECT users.display_name,
                   sum(prediction) as prediction,
                   sum(favourites) as favourites,
                   sum(time_of_first_goal) as time_of_first_goal,
                   sum(total) as score
            FROM users
                 INNER JOIN user_match_points ON users.user_id = user_match_points.user_id
                 INNER JOIN matches ON user_match_points.match_id = matches.match_id
            WHERE matches.time <= $1
            GROUP BY users.user_id
            ORDER BY sum(total) DESC
            ",
            ).bind::<Timestamptz, _>(up_to_chrono)
                .load(&self.connection)?;

            let time = {
                use diesel::dsl::sql;
                use schema::match_outcomes::table as match_outcomes;
                use schema::matches::dsl::*;

                matches
                    .select(sql::<Nullable<Timestamptz>>("max(time) as time"))
                    .inner_join(match_outcomes)
                    .filter(sql("time < ").bind::<Timestamptz, _>(up_to_chrono))
                    .first(&self.connection)?
            };

            Ok((leaders, time))
        } else {
            let leaders = sql_query(
                "
            SELECT users.display_name,
                   sum(prediction) as prediction,
                   sum(favourites) as favourites,
                   sum(time_of_first_goal) as time_of_first_goal,
                   sum(total) as score
            FROM users
                 INNER JOIN user_match_points ON users.user_id = user_match_points.user_id
            GROUP BY users.user_id
            ORDER BY sum(total) DESC
            ",
            ).load(&self.connection)?;

            let time = {
                use diesel::dsl::{max, sql};
                use schema::match_outcomes::table as match_outcomes;
                use schema::matches::dsl::*;

                matches
                    .select(sql::<Nullable<Timestamptz>>("max(time) as time"))
                    .inner_join(match_outcomes)
                    .first(&self.connection)?
            };

            Ok((leaders, time))
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
pub fn index(
    (auth, query, state): (CurrentUser, Query<FetchLeaderBoard>, State<AppState>),
) -> impl Responder {
    let data = query.into_inner();

    state
        .db
        .send(data.clone())
        .and_then(move |res| {
            Ok(match res {
                Ok((leader_board, previous)) => {
                    let mut context = Context::new();
                    context.add("current_user", &auth.current_user);
                    context.add("leader_board", &leader_board);
                    context.add(
                        "current",
                        &data.up_to.map(|unix| {
                            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(unix, 0), Utc)
                        }),
                    );
                    context.add("previous", &previous);

                    let rendered = TEMPLATE_SERVICE.render("scores/index.html", &context);
                    match rendered {
                        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
                        Err(error) => {
                            println!("{:?}", error);
                            HttpResponse::InternalServerError()
                                .content_type("text/html")
                                .body("Something went wrong")
                        }
                    }
                }
                Err(error) => {
                    println!("{:?}", error);
                    HttpResponse::InternalServerError()
                        .content_type("text/html")
                        .body("Something went very wrong")
                }
            })
        })
        .responder()
}
