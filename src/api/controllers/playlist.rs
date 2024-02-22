use std::sync::Arc;

use actix_web::{get, post, web, HttpResponse, Responder, Result, Scope};
use diesel::{r2d2::{ConnectionManager, Pool}, SqliteConnection};
use serenity::{all::GuildId, client::Cache};
use songbird::Songbird;

use crate::bot::utils::{player::{initialize_guild_player_web, PlayerDataType, PlayerState, Position}, track::{Track, WebTrack}};

#[post("/")] // <- define path parameters
async fn add(path: web::Path<u64>, trak: web::Json<WebTrack>, songbird: web::Data<Arc<Songbird>>, cache: web::Data<Arc<Cache>>, player_data: web::Data<PlayerDataType>, pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>) -> Result<impl Responder> {
    let guild_id = path.into_inner();
    let guild_id = GuildId::from(guild_id);

    let guild_id = match cache.guild(guild_id) {
        Some(guild) => guild.id,
        None => return Ok(HttpResponse::NotFound().body("Guild not found"))
    };

    initialize_guild_player_web(player_data.as_ref(), pool.get(), guild_id).await;

    match songbird.get(guild_id) {
        Some(handler) => {
            let handler = handler.lock().await;
            match handler.current_channel() {
                Some(_) => {},
                None => return Ok(HttpResponse::Conflict().body("Join a channel first"))
            }
        },
        None => return Ok(HttpResponse::Conflict().body("Join a channel first"))
    };

    let player = player_data.read().await;
    let player = player.get(&guild_id.get()).unwrap();
    let mut last_id = player.playlist_sync_and_last_id.lock().await;

    *last_id+=1;

    let track = Track::from_web(trak.into_inner(), last_id.clone());
    let mut player_playlist = player.playlist.write().await;
    let mut state = player.state.write().await;

    match *state {
        PlayerState::Ended => {
            let mut last_updated_position = player.position.write().await;
            player_playlist.current = Some(track.clone());
            *state = PlayerState::Playing;
            *last_updated_position = Position::default();

            let mut child = track.get_child_web(player_data.clone().into_inner(), &guild_id.get(), 0.0).await.unwrap();
            let stdin = child.stdin.take().unwrap();
            let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

            if let Some(handler_lock) = songbird.get(guild_id) {
                let mut handler = handler_lock.lock().await;
                let mut ffmpeg = player.ffmpeg.write().await;
                let mut player_handler = player.player.write().await;

                let _ = ffmpeg.insert(stdin);

                let handle = handler.play_only_input(data);
                let _ = player_handler.insert(handle);
            }
        },
        _ => {
            player_playlist.tracks.push_back(track.clone());
        }
    }
    Ok(HttpResponse::Ok().json(track))
}

pub fn api_scope() -> Scope {
    web::scope("/{guild_id}/playlist")
        .service(add)
}