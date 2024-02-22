use std::{sync::Arc, time::Duration};

use actix_web::{get, post, web, HttpResponse, Responder, Result, Scope};
use diesel::{r2d2::{ConnectionManager, Pool}, SqliteConnection};
use serenity::{all::GuildId, client::Cache};

use crate::bot::utils::{player::{initialize_guild_player_web, PlayerDataType, PlayerState, Position}, track::{Track, WebTrack}};

#[get("")]
async fn get(path: web::Path<u64>, cache: web::Data<Arc<Cache>>, player_data: web::Data<PlayerDataType>, pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>) -> Result<impl Responder> {
    let guild_id = GuildId::from(path.into_inner());
    let guild_id = match cache.guild(guild_id) {
        Some(guild) => guild.id,
        None => return Ok(HttpResponse::NotFound().body("Guild not found"))
    };

    initialize_guild_player_web(player_data.as_ref(), pool.get(), guild_id).await;

    let player = player_data.read().await;
    let player = player.get(&guild_id.get()).unwrap();
    let settings = player.settings.read().await;
    let player_handler = player.player.read().await.clone();
    let position = player.position.read().await;

    match player_handler {
        Some(player_handler) => match player_handler.get_info().await {
            Ok(data) => {
                let res = position.last_position + Duration::from_secs_f64((data.position - position.last_player_position).as_secs_f64() * settings.speed + 0.2);
                Ok(HttpResponse::Ok().json(res))
            },
            _ => Ok(HttpResponse::InternalServerError().body("Error getting player info"))
        },
        None => Ok(HttpResponse::NotFound().body("Player handler not found"))
    }
}

#[post("/{value}")] // <- define path parameters
async fn seek(path: web::Path<(u64, f64)>, cache: web::Data<Arc<Cache>>, player_data: web::Data<PlayerDataType>, pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>) -> Result<impl Responder> {
    let (guild_id, value) = path.into_inner();
    let guild_id = GuildId::from(guild_id);

    let guild_id = match cache.guild(guild_id) {
        Some(guild) => guild.id,
        None => return Ok(HttpResponse::NotFound().body("Guild not found"))
    };

    initialize_guild_player_web(player_data.as_ref(), pool.get(), guild_id).await;

    let player = player_data.read().await;
    let player = player.get(&guild_id.get()).unwrap();

    let mut state = player.state.write().await;
    let player_handler = player.player.write().await.clone();

    match *state {
        PlayerState::Playing | PlayerState::Paused => match player_handler {
            Some(handler) => match handler.stop() {
                Ok(_) => {
                    let mut position = player.position.write().await;
                    *state = PlayerState::Seeking;
                    *position = Position::from_secs_f64(value);
                    Ok(HttpResponse::Ok().body("Seeked"))
                },
                _ => Ok(HttpResponse::InternalServerError().body("Failed to seek"))
            },
            None => Ok(HttpResponse::InternalServerError().body("Player not found"))
        },
        _ => Ok(HttpResponse::InternalServerError().body("Player not found"))
    }
}

pub fn api_scope() -> Scope {
    web::scope("/{guild_id}/seek")
        .service(seek)
        .service(get)
}