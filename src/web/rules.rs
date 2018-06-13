use actix_web::{HttpResponse, Responder};
use templates::{Context, TEMPLATE_SERVICE};

pub fn show() -> impl Responder {
    let context = Context::new();

    let rendered = TEMPLATE_SERVICE.render("rules.html", &context);
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
