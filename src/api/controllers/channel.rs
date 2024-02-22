use std::sync::Arc;

use actix_web::{get, web, HttpResponse, Responder, Result, Scope};
use diesel::{r2d2::{ConnectionManager, Pool}, SqliteConnection};
use serenity::{all::{ChannelId, ChannelType, GuildId}, client::Cache};
use songbird::{driver::Bitrate, CoreEvent, Event, Songbird, TrackEvent};

use crate::bot::{events::TrackEndNotifierWeb, utils::player::{initialize_guild_player_web, PlayerDataType}};

#[get("/join/{channel_id}")] // <- define path parameters
async fn join(path: web::Path<(u64, u64)>, songbird: web::Data<Arc<Songbird>>, cache: web::Data<Arc<Cache>>, player: web::Data<PlayerDataType>, pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>) -> Result<impl Responder> {
    let (guild_id, channel_id) = path.into_inner();
    let guild_id = GuildId::from(guild_id);

    let (guild_id, channel_id) = match cache.guild_channels(guild_id) {
        Some(channels) => {
            match channels.get(&ChannelId::from(channel_id)) {
                Some(channel) => match channel.kind {
                    ChannelType::Voice => (guild_id, channel.id),
                    _ => return Ok(HttpResponse::BadRequest().body("Invalid channel type"))
                },
                None => return Ok(HttpResponse::NotFound().body("Channel not found"))
            }
        },
        None => return Ok(HttpResponse::NotFound().body("Guild not found"))
    };

    initialize_guild_player_web(player.as_ref(), pool.get(), guild_id).await;

    return match songbird.get(guild_id) {
        Some(handler) => {
            let mut handler = handler.lock().await;
            match handler.join(channel_id).await {
                Ok(_) => Ok(HttpResponse::Ok().body("ok")),
                Err(e) => Ok(HttpResponse::InternalServerError().body(format!("Failed to join channel: {}", e)))
            }
        },
        None => match songbird.join(guild_id, channel_id).await {
            Ok(handler) => {
                let mut handler = handler.lock().await;
                handler.set_bitrate(Bitrate::BitsPerSecond(256000));
                handler.remove_all_global_events();
                handler.add_global_event(
                    Event::Track(TrackEvent::End),
                    TrackEndNotifierWeb {
                        guild_id: guild_id,
                        songbird: songbird.clone().into_inner(),
                        player: player.clone().into_inner()
                    }
                );
                handler.add_global_event(
                    Event::Core(CoreEvent::DriverConnect),
                    TrackEndNotifierWeb {
                        guild_id: guild_id,
                        songbird: songbird.into_inner(),
                        player: player.into_inner()
                    }
                );
                Ok(HttpResponse::Ok().body("ok"))
            },
            Err(e) => Ok(HttpResponse::InternalServerError().body(format!("Failed to join channel: {}", e)))
        }
    };
}

pub fn api_scope() -> Scope {
    web::scope("/{guild_id}/channel")
        .service(join)
}