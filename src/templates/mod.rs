pub use tera::{Context, Tera};

lazy_static! {
    pub static ref TEMPLATE_SERVICE: Tera = {
        let tera = compile_templates!("templates/**/*");
        tera
    };
}
