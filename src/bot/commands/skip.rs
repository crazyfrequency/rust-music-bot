use std::collections::HashMap;

use serenity::all::ResolvedValue;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::{check_msg, get_title_author_str};
use crate::bot::utils::player::{PlayerData, PlayerState, initialize_guild_player};

pub async fn run(ctx: Context, command: CommandInteraction) {
    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;
    
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();
    let track_id = match options.get("track") {
        Some(ResolvedValue::String(id)) => match id.parse::<u64>() {
            Ok(id) => Some(id),
            Err(_) => {
                let text = match command.locale.as_str() {
                    "ru" => "Не удалось получить id трека!",
                    _ => "Could not get track id!"
                };
                let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
                let builder = CreateInteractionResponse::Message(data);
                check_msg(command.create_response(&ctx.http, builder).await);
                return ;
            }
        },
        _ => None
    };

    let locale = command.locale.as_str();
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();

    let mut player_playlist = player.playlist.write().await;
    let mut state = player.state.write().await;
    let player_handler = player.player.write().await.clone();

    let text = match *state {
        PlayerState::Playing | PlayerState::Paused | PlayerState::Seeking => match track_id {
            Some(track_id) => match player_playlist.current.clone() {
                Some(track) => match track.id == track_id{
                    true => match player_handler {
                        Some(handler) => match handler.stop() {
                            Ok(_) => {
                                *state = PlayerState::InSkip;
                                match locale {
                                    "ru" => format!("Пропущен: {}", get_title_author_str(&track, locale)),
                                    _ => format!("Skipped: {}", get_title_author_str(&track, locale))
                                }
                            },
                            Err(_) => match locale {
                                "ru" => format!("Произошла ошибка при пропуске трека!"),
                                _ => format!("An error occurred while skipping the track!")
                            }
                        },
                        None => match locale {
                            "ru" => format!("Не удалось получить плеер!"),
                            _ => format!("Failed to get player!")
                        }
                    },
                    false => match player_playlist.tracks.iter().position(|track| track.id == track_id) {
                        Some(index) => {
                            player_playlist.tracks.remove(index);
                            match locale {
                                "ru" => format!("Пропущен: {}", get_title_author_str(&track, locale)),
                                _ => format!("Skipped: {}", get_title_author_str(&track, locale))
                            }
                        },
                        None => match locale {
                            "ru" => format!("Не удалось найти трек!"),
                            _ => format!("Failed to find track!")
                        }
                    }
                },
                None => match player_playlist.tracks.iter().position(|track| track.id == track_id) {
                    Some(index) => match player_playlist.tracks.remove(index) {
                        Some(track) => match locale {
                            "ru" => format!("Пропущен: {}", get_title_author_str(&track, locale)),
                            _ => format!("Skipped: {}", get_title_author_str(&track, locale))
                        },
                        None => match locale {
                            "ru" => format!("Не удалось найти трек!"),
                            _ => format!("Failed to find track!")
                        }
                    },
                    None => match locale {
                        "ru" => format!("Не удалось найти трек!"),
                        _ => format!("Failed to find track!")
                    }
                }
            },
            None => match player_handler {
                Some(handler) => match handler.stop() {
                    Ok(_) =>{
                        *state = PlayerState::InSkip;
                        match player_playlist.current.clone() {
                            Some(track) => match locale {
                                "ru" => format!("Пропущен: {}", get_title_author_str(&track, locale)),
                                _ => format!("Skipped: {}", get_title_author_str(&track, locale))
                            },
                            None => match locale {
                                "ru" => format!("Пропущен текущий трек"),
                                _ => format!("Skipped current track")
                            }
                        }
                    },
                    Err(_) => match locale {
                        "ru" => format!("Произошла ошибка при пропуске трека!"),
                        _ => format!("An error occurred while skipping the track!")
                    }
                },
                None => match locale {
                    "ru" => format!("Не удалось получить плеер!"),
                    _ => format!("Failed to get player!")
                }
            }
        } 
        _ => match locale {
            "ru" => format!("Не удалось получить плеер!"),
            _ => format!("Failed to get player!")
        }
    };

    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("skip")
        .description("Skips a track")
        .description_localized("ru", "Пропускает трек")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "track", "Track to skip(0 - current track)")
                .description_localized("ru", "Трек для пропуска(0 - текущий трек)")
                .set_autocomplete(true)
        ).dm_permission(false)
}