use std::collections::HashMap;
use std::time::Duration;

use serenity::all::ResolvedValue;
use serenity::builder::{CreateCommand, CreateCommandOption, EditInteractionResponse, CreateEmbed};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};
use songbird::{TrackEvent, Event, CoreEvent};
use songbird::driver::Bitrate;
use tokio::time::sleep;
use youtube_dl::SearchType;

use crate::bot::events::TrackEndNotifier;
use crate::bot::utils::parser::{parse_url, ParsedDataType, parse_track_yt, search_track_yt, search_track_vk};
use crate::bot::utils::player::{PlayerData, initialize_guild_player, PlayerState, Position};
use crate::bot::utils::track::{Track, PlaylistType};
use crate::bot::utils::{get_voice_channel, check_msg};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();
    println!("{:?}", options);
    let url = match options.get("url") {
        Some(ResolvedValue::String(url)) => Some(*url),
        _ => None
    }.expect("url option parse error");
    let limit = match options.get("limit") {
        Some(ResolvedValue::Integer(limit)) => *limit,
        _ => 25
    } as usize;
    let search_type = match options.get("search") {
        Some(ResolvedValue::Integer(value)) => Some(*value),
        _ => None
    };
    check_msg(command.defer(&ctx.http).await);

    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;

    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();
    let mut last_id = player.playlist_sync_and_last_id.lock().await;

    let manager = songbird::get(&ctx).await.expect("Songbird Voice client placed in at initialisation.").clone();
    
    match manager.get(command.guild_id.unwrap()) {
        Some(handler) => {
            let mut handler = handler.lock().await;
            match handler.current_channel() {
                Some(_) => {},
                None => {
                    let (channel, message) = get_voice_channel(&ctx, &command).await;
                    if channel.is_none() {
                        let builder = EditInteractionResponse::new()
                            .content(message.unwrap());
                        check_msg(command.edit_response(&ctx.http, builder).await);
                        return;
                    };
                    let channel = channel.unwrap();
                    if let Ok(_) = handler.join(channel).await{
                        let text = match command.locale.as_str() {
                            "ru" => format!("Подключился к \"<#{}>\"", channel.get()),
                            _ => format!("Connected to \"<#{}>\"", channel.get())
                        };
                        let builder = EditInteractionResponse::new()
                            .content(text);
                        check_msg(command.edit_response(&ctx.http, builder).await);
                    };
                    handler.set_bitrate(Bitrate::BitsPerSecond(320000));
                }
            }
        },
        None => {
            let (channel, message) = get_voice_channel(&ctx, &command).await;
            if channel.is_none() {
                let builder = EditInteractionResponse::new()
                    .content(message.unwrap());
                check_msg(command.edit_response(&ctx.http, builder).await);
                return;
            };

            let channel = channel.unwrap();
            match manager.join(command.guild_id.unwrap(), channel).await {
                Ok(handler) => {
                    let text = match command.locale.as_str() {
                        "ru" => format!("Подключился к \"<#{}>\"", channel.get()),
                        _ => format!("Connected to \"<#{}>\"", channel.get())
                    };
                    let builder = EditInteractionResponse::new().content(text);
                    check_msg(command.edit_response(&ctx.http, builder).await);
                    let mut handler = handler.lock().await;
                    let _ = handler.deafen(true).await;
                    handler.set_bitrate(Bitrate::BitsPerSecond(256000));
                    handler.remove_all_global_events();
                    handler.add_global_event(
                        Event::Track(TrackEvent::End),
                        TrackEndNotifier {
                            guild_id: command.guild_id.unwrap(),
                            ctx_clone: ctx.clone()
                        }
                    );
                    handler.add_global_event(
                        Event::Core(CoreEvent::DriverConnect),
                        TrackEndNotifier {
                            guild_id: command.guild_id.unwrap(),
                            ctx_clone: ctx.clone()
                        }
                    )
                },
                Err(e) => {
                    println!("Error joining {:?}", e);
                    let text = match command.locale.as_str() {
                        "ru" => "Ошибка подключения!",
                        _ => "Connection error!"
                    };
                    let builder = EditInteractionResponse::new().content(text);
                    check_msg(command.edit_response(&ctx.http, builder).await);
                    return;
                }
            }
        }
    }

    let mut playlist: PlaylistType = PlaylistType::None;
    *last_id+=1;
    let not_query = url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ftp://");
    let track = if not_query {
        match parse_url(url, *last_id).await {
            ParsedDataType::Track(track) => Some(track),
            ParsedDataType::YtDlPlaylist((parsed_playlist, track)) => {
                playlist = PlaylistType::YtDl(parsed_playlist);
                Some(track)
            },
            ParsedDataType::VkPlaylist((parsed_playlist, track)) => {
                playlist = PlaylistType::Vk(parsed_playlist);
                Some(track)
            },
            ParsedDataType::None => None
        }
    } else {
        println!("{:?}", search_type);
        match search_type {
            Some(1) => search_track_yt(url, *last_id, SearchType::SoundCloud).await,
            Some(2) => search_track_vk(url, *last_id).await,
            _ => search_track_yt(url, *last_id, SearchType::Youtube).await
        }
    };
    if track.is_none() {
        let builder = EditInteractionResponse::new().content(match command.locale.as_str() {
            "ru" => "Произошла ошибка во время обработки!",
            _ => "An error occurred while processing!"
        });
        check_msg(command.edit_response(&ctx.http, builder).await);
        return ;
    }
    let track = track.unwrap();
    
    let builder = match &playlist {
        PlaylistType::YtDl(playlist) => {
            let locale = command.locale.as_str();
            let embed = CreateEmbed::new()
                .color(14441063)
                .title(match locale {
                    "ru" => "Добавлен плейлист:",
                    _ => "Added playlist:"
                });
            
            EditInteractionResponse::new().embed(embed)
        },
        PlaylistType::Vk(playlist) => {
            let locale = command.locale.as_str();
            let embed = CreateEmbed::new()
                .color(14441063)
                .title(match locale {
                    "ru" => "Добавлен плейлист:",
                    _ => "Added playlist:"
                });
            
            EditInteractionResponse::new().embed(embed)
        }
        PlaylistType::None => {
            let locale = command.locale.as_str();
            let embed = track.get_embed(command.locale.as_str())
                .color(14441063)
                .title(match locale {
                    "ru" => "Добавлен трек:",
                    _ => "Added track:"
                });
            EditInteractionResponse::new().embed(embed)
        }
    };
    check_msg(command.edit_response(&ctx.http, builder).await);

    println!("{:#?}", track);
    {
        let mut player_playlist = player.playlist.write().await;
        let mut state = player.state.write().await;
        match *state {
            PlayerState::Ended => {
                let mut last_updated_position = player.position.write().await;
                player_playlist.current = Some(track.clone());
                *state = PlayerState::Playing;
                *last_updated_position = Position::default();

                let mut child = track.get_child(&ctx, &command.guild_id.unwrap().get(), 0.0).await.unwrap();
                let stdin = child.stdin.take().unwrap();
                let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                if let Some(handler_lock) = manager.get(command.guild_id.unwrap()) {
                    let mut handler = handler_lock.lock().await;
                    let mut ffmpeg = player.ffmpeg.write().await;
                    let mut player_handler = player.player.write().await;

                    let _ = ffmpeg.insert(stdin);

                    let handle = handler.play_only_input(data);
                    let _ = player_handler.insert(handle);
                }
            },
            _ => {
                player_playlist.tracks.push_back(track);
            }
        }
    }

    match &playlist {
        PlaylistType::YtDl(playlist) => {
            for url in playlist.tracks.iter().skip(1).take(limit-1) {
                match youtube_dl::YoutubeDl::new(url).flat_playlist(true).socket_timeout("15").run_raw_async().await {
                    Ok(src) => {
                        *last_id+=1;
                        match parse_track_yt(src, *last_id).await {
                            Some(track) => {
                                let mut player_playlist = player.playlist.write().await;
                                let mut state = player.state.write().await;
                                match *state {
                                    PlayerState::Ended => {
                                        let mut last_updated_position = player.position.write().await;
                                        player_playlist.current = Some(track.clone());
                                        *state = PlayerState::Playing;
                                        *last_updated_position = Position::default();
                        
                                        let mut child = track.get_child(&ctx, &command.guild_id.unwrap().get(), 0.0).await.unwrap();
                                        let stdin = child.stdin.take().unwrap();
                                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));
                        
                                        if let Some(handler_lock) = manager.get(command.guild_id.unwrap()) {
                                            let mut handler = handler_lock.lock().await;
                                            let mut ffmpeg = player.ffmpeg.write().await;
                                            let mut player_handler = player.player.write().await;
                        
                                            let _ = ffmpeg.insert(stdin);
                        
                                            let handle = handler.play_only_input(data);
                                            let _ = player_handler.insert(handle);
                                        }
                                    },
                                    _ => player_playlist.tracks.push_back(track)
                                }
                            },
                            None => {}
                        };
                    },
                    Err(_) => {}
                };
                sleep(Duration::from_millis(200)).await;
            }
        },
        PlaylistType::Vk(playlist) => {
            for track in playlist.tracks.iter().skip(1).take(limit-1) {
                *last_id+=1;
                let track = Track::from_vk(track.clone(), *last_id);
                let mut player_playlist = player.playlist.write().await;
                let mut state = player.state.write().await;
                match *state {
                    PlayerState::Ended => {
                        let mut last_updated_position = player.position.write().await;
                        player_playlist.current = Some(track.clone());
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();
        
                        let mut child = track.get_child(&ctx, &command.guild_id.unwrap().get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));
        
                        if let Some(handler_lock) = manager.get(command.guild_id.unwrap()) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;
                            let mut player_handler = player.player.write().await;
        
                            let _ = ffmpeg.insert(stdin);
        
                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                    },
                    _ => player_playlist.tracks.push_back(track)
                }
            }
        },
        _ => {}
    }
    
}

pub fn register() -> CreateCommand {
    CreateCommand::new("play")
        .description("Plays/adds track(s) to the playlist")
        .description_localized("ru", "Проигрывает/добавляет в плейлист трек(и)")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "url", "URL or search query")
                .description_localized("ru", "Ссылка или запрос")
                .required(true)
        ).add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "limit", "Limit of tracks to add(default 25)")
                .description_localized("ru", "Лимит добавляемых треков(по умолчанию 25)")
                .min_int_value(1)
                .max_int_value(35)
                .required(false)
        ).add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "search", "Search type(default \"YouTube)\"")
                .description_localized("ru", "Тип поиска(по умолчанию \"YouTube)\"")
                .add_int_choice("YouTube", 0)
                .add_int_choice("SoundCloud", 1)
                .add_int_choice("VK", 2)
                .required(false),
        ).dm_permission(false)
}
