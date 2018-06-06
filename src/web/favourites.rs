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

struct FavouriteInfo {
    current_selection: Vec<(Favourite, Option<Country>)>,
    available_countries: Vec<Country>,
}

struct FetchFavouriteInfo {
    user_id: i32,
}

impl Message for FetchFavouriteInfo {
    type Result = Result<FavouriteInfo, failure::Error>;
}

impl Handler<FetchFavouriteInfo> for DbExecutor {
    type Result = Result<FavouriteInfo, failure::Error>;

    fn handle(&mut self, msg: FetchFavouriteInfo, _: &mut Self::Context) -> Self::Result {
        let available_countries = {
            use schema::countries::dsl::*;

            countries.order(name.asc()).load(&self.connection)?
        };
        let mut current_selection = {
            use schema::countries;
            use schema::favourites::dsl::*;

            favourites
                .filter(user_id.eq(msg.user_id))
                .order(choice)
                .left_join(countries::table)
                .load(&self.connection)?
        };

        if current_selection.len() < 4 {
            for i in (current_selection.len() + 1)..=4 {
                current_selection.push((
                    Favourite {
                        user_id: msg.user_id,
                        country_id: None,
                        choice: i as i16,
                        created_at: Utc::now().naive_local(), // Doesn't matter too much if this is the right method (as opposed to naive_utc)
                        updated_at: Utc::now().naive_local(),
                        phase: 0,
                    },
                    None,
                ));
            }
        }

        Ok(FavouriteInfo {
            available_countries,
            current_selection,
        })
    }
}

fn render_favourite_selection(auth: CurrentUser, fav_info: FavouriteInfo) -> HttpResponse {
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

pub fn edit(auth: CurrentUser, req: HttpRequest<AppState>) -> impl Responder {
    req.state()
        .db
        .send(FetchFavouriteInfo {
            user_id: auth.current_user.user_id,
        })
        .and_then(|fav_info| match fav_info {
            Ok(info) => Ok(render_favourite_selection(auth, info)),
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
}

impl Message for UpdatedFavouriteInfo {
    type Result = Result<(), failure::Error>;
}

impl Handler<UpdatedFavouriteInfo> for DbExecutor {
    type Result = Result<(), failure::Error>;

    fn handle(&mut self, msg: UpdatedFavouriteInfo, _: &mut Self::Context) -> Self::Result {
        let data = vec![
            msg.data.fav_1,
            msg.data.fav_2,
            msg.data.fav_3,
            msg.data.fav_4,
        ];

        self.connection
            .transaction::<_, diesel::result::Error, _>(|| {
                {
                    use schema::favourites::dsl::*;
                    diesel::update(favourites.filter(user_id.eq(msg.user_id)))
                        .set((country_id.eq::<Option<i32>>(None),))
                        .execute(&self.connection)?;
                }
                for (&country_id, choice_idx) in data.iter().zip((1..=4).into_iter()).into_iter() {
                    let favourite = UpdatedFavourite {
                        user_id: msg.user_id,
                        country_id: if country_id == 0 {
                            None
                        } else {
                            Some(country_id)
                        },
                        phase: 0,
                        choice: choice_idx,
                    };

                    {
                        use diesel::insert_into;
                        use schema::favourites::dsl::*;

                        insert_into(favourites)
                            .values(&favourite)
                            .on_conflict((user_id, phase, choice))
                            .do_update()
                            .set(&favourite)
                            .execute(&self.connection)?;
                    }
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
    fav_2: i32,
    fav_3: i32,
    fav_4: i32,
}

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
