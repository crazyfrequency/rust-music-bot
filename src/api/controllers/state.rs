use std::{sync::Arc, time::Duration};

use actix_web::{get, post, web, HttpResponse, Responder, Result, Scope};
use diesel::{r2d2::{ConnectionManager, Pool}, SqliteConnection};
use serenity::{all::{ChannelId, ChannelType, GuildId}, client::Cache, model::guild};
use songbird::{driver::Bitrate, CoreEvent, Event, Songbird, TrackEvent};

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
    let state = player.state.read().await;

    Ok(HttpResponse::Ok().json(match *state {
        PlayerState::Paused => "paused",
        PlayerState::Ended => "ended",
        _ => "playing"
    }))
}

#[post("/resume")] // <- define path parameters
async fn resume(path: web::Path<u64>, cache: web::Data<Arc<Cache>>, player_data: web::Data<PlayerDataType>, pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>) -> Result<impl Responder> {
    let guild_id = path.into_inner();
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
            Some(handler) => match handler.play() {
                Ok(_) => {
                    *state = PlayerState::Playing;
                    Ok(HttpResponse::Ok().body("Resumed playing"))
                },
                _ => Ok(HttpResponse::InternalServerError().body("Failed to resume"))
            },
            None => Ok(HttpResponse::Conflict().body("Player is not playing"))
        },
        _ => Ok(HttpResponse::Conflict().body("Player is not playing"))
    }
}

#[post("/pause")] // <- define path parameters
async fn pause(path: web::Path<u64>, cache: web::Data<Arc<Cache>>, player_data: web::Data<PlayerDataType>, pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>) -> Result<impl Responder> {
    let guild_id = path.into_inner();
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
            Some(handler) => match handler.pause() {
                Ok(_) => {
                    *state = PlayerState::Paused;
                    Ok(HttpResponse::Ok().body("Paused playing"))
                },
                _ => Ok(HttpResponse::InternalServerError().body("Failed to pause"))
            },
            None => Ok(HttpResponse::Conflict().body("Player is not playing"))
        },
        _ => Ok(HttpResponse::Conflict().body("Player is not playing"))
    }
}

pub fn api_scope() -> Scope {
    web::scope("/{guild_id}/state")
        .service(get)
        .service(resume)
        .service(pause)
}