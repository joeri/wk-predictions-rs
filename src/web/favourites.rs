use models::{Country, Favourite, UpdatedFavourite};
use templates::{Context, TEMPLATE_SERVICE};
use web::app_state::DbExecutor;
use web::{app_state::AppState, auth::CurrentUser};

use actix::prelude::*;
use actix_web::{AsyncResponder, Form, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use diesel::{self, prelude::*};
use failure;
use futures::Future;
use std::{error::Error as StdError, fmt};

struct FavouriteInfo {
    current_selection: Vec<(Favourite, Option<Country>)>,
    available_countries: Vec<Country>,
}

struct FetchFavouriteInfo {
    user_id: i32,
    phase: i16,
}

impl Message for FetchFavouriteInfo {
    type Result = Result<FavouriteInfo, failure::Error>;
}

#[derive(Debug)]
struct UnexpectedError;

impl fmt::Display for UnexpectedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unexpected error")
    }
}

impl StdError for UnexpectedError {
    fn description(&self) -> &str {
        "Unexpected error"
    }
}

impl Handler<FetchFavouriteInfo> for DbExecutor {
    type Result = Result<FavouriteInfo, failure::Error>;

    fn handle(&mut self, msg: FetchFavouriteInfo, _: &mut Self::Context) -> Self::Result {
        let available_countries = {
            use schema::countries::dsl::*;

            if msg.phase == 0 {
                countries.order(name.asc()).load(&self.connection)?
            } else if msg.phase == 1 {
                countries
                    .filter(qualified_for_knockout.eq(true))
                    .order(name.asc())
                    .load(&self.connection)?
            } else {
                use schema::{countries, match_participants};

                countries
                    .left_join(match_participants::table)
                    .filter(match_participants::columns::stage_id.eq(4))
                    .order(name.asc())
                    .select(countries::all_columns)
                    .load(&self.connection)?
            }
        };
        let mut current_selection = {
            use schema::countries;
            use schema::favourites::dsl::*;

            favourites
                .filter(user_id.eq(msg.user_id))
                .filter(phase.eq(msg.phase))
                .order(choice)
                .left_join(countries::table)
                .load(&self.connection)?
        };

        let (offset, length) = match msg.phase {
            0 => (1, 4),
            1 => (5, 3),
            2 => (8, 1),
            _ => unreachable!(),
        };

        if current_selection.is_empty() {
            for i in offset..(offset + length) {
                current_selection.push((
                    Favourite {
                        user_id: msg.user_id,
                        country_id: None,
                        choice: i as i16,
                        phase: msg.phase,
                        source: "manual".to_string(),

                        // Doesn't matter too much if naive_local is the right method
                        // (as opposed to naive_utc), because we don't send it to the DB here,
                        // we only need it to get the Favourite type to display things
                        created_at: Utc::now().naive_local(),
                        updated_at: Utc::now().naive_local(),
                    },
                    None,
                ));
            }
        }
        if current_selection.len() != length {
            Err(UnexpectedError.into())
        } else {
            Ok(FavouriteInfo {
                available_countries,
                current_selection,
            })
        }
    }
}

fn render_favourite_selection(auth: &CurrentUser, fav_info: &FavouriteInfo) -> HttpResponse {
    let mut context = Context::new();
    context.add("current_user", &auth.current_user);
    context.add("current_selection", &fav_info.current_selection);
    context.add("available_countries", &fav_info.available_countries);

    let rendered = TEMPLATE_SERVICE.render("favourites/edit.html", &context);

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

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
pub fn edit(auth: CurrentUser, req: HttpRequest<AppState>) -> impl Responder {
    req.state()
        .db
        .send(FetchFavouriteInfo {
            user_id: auth.current_user.user_id,
            phase: 2,
        })
        .and_then(move |fav_info| match fav_info {
            Ok(info) => Ok(render_favourite_selection(&auth, &info)),
            Err(err) => {
                println!("{:?}", err);
                Ok(HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body("An error occurred"))
            }
        })
        .responder()
}

struct UpdatedFavouriteInfo {
    user_id: i32,
    data: FavouriteSelectionForm,
    phase: i16,
}

impl Message for UpdatedFavouriteInfo {
    type Result = Result<(), failure::Error>;
}

impl Handler<UpdatedFavouriteInfo> for DbExecutor {
    type Result = Result<(), failure::Error>;

    fn handle(&mut self, msg: UpdatedFavouriteInfo, _: &mut Self::Context) -> Self::Result {
        let data = vec![msg.data.fav_1];

        self.connection
            .transaction::<_, diesel::result::Error, _>(|| {
                let mut changes = Vec::new();
                for (&country_id, choice_idx) in data.iter().zip((8..=8).into_iter()) {
                    changes.push(UpdatedFavourite {
                        user_id: msg.user_id,
                        country_id: if country_id == 0 {
                            None
                        } else {
                            Some(country_id)
                        },
                        phase: msg.phase,
                        choice: choice_idx,
                    });
                }

                {
                    use diesel::insert_into;
                    use diesel::pg::upsert::excluded;
                    use schema::favourites::dsl::*;

                    insert_into(favourites)
                        .values(&changes)
                        .on_conflict((user_id, phase, choice))
                        .do_update()
                        .set(country_id.eq(excluded(country_id)))
                        .execute(&self.connection)?;
                }

                Ok(())
            })?;

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
pub struct FavouriteSelectionForm {
    // Why not use a Vec? Because Form uses serde_urlencoded to deserialize,
    // and serde_deserialize decided that the rack like behaviour that converts
    // values into strings doesn't belong in that crate but in serde_qs, I could
    // write my own extractor based on that crate, but at the moment I'm trying
    // to get something working
    fav_1: i32,
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
pub fn update(
    auth: CurrentUser,
    form: Form<FavouriteSelectionForm>,
    req: HttpRequest<AppState>,
) -> impl Responder {
    req.state()
        .db
        .send(UpdatedFavouriteInfo {
            user_id: auth.current_user.user_id,
            data: form.into_inner(),
            phase: 2,
        })
        .and_then(|update| match update {
            Ok(()) => Ok(HttpResponse::SeeOther().header("Location", "/").finish()),
            Err(err) => {
                println!("{:?}", err);
                Ok(HttpResponse::InternalServerError()
                    .content_type("text/html")
                    .body("An error occurred"))
            }
        })
        .responder()
}
