use actix_web::{web, Scope};

mod channel;
mod playlist;
mod seek;
mod state;

pub fn api_scope() -> Scope {
    web::scope("/api")
        .service(channel::api_scope())
        .service(playlist::api_scope())
        .service(seek::api_scope())
        .service(state::api_scope())
}