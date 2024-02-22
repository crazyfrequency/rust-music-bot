use std::collections::HashMap;

use serenity::all::{ResolvedValue, ChannelType};
use serenity::builder::{CreateCommand, CreateCommandOption, EditInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};
use songbird::{TrackEvent, Event, CoreEvent};
use songbird::driver::Bitrate;

use crate::bot::events::TrackEndNotifier;
use crate::bot::utils::{check_msg, get_voice_channel};
use crate::bot::utils::player::initialize_guild_player;

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();
    let channel = match options.get("channel") {
        Some(ResolvedValue::Channel(channel)) => Some(channel),
        _ => None
    };
    check_msg(command.defer_ephemeral(&ctx.http).await);

    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;

    let manager = songbird::get(&ctx).await.expect("Songbird Voice client placed in at initialisation.").clone();

    let text = match manager.get(command.guild_id.unwrap()) {
        Some(handler) => {
            let mut handler = handler.lock().await;
            match channel {
                Some(channel) => {
                    match handler.join(channel.id).await {
                        Ok(_) => match command.locale.as_str() {
                            "ru" => format!("Подключился к \"<#{}>\"", channel.id.get()),
                            _ => format!("Connected to \"<#{}>\"", channel.id.get())
                        },
                        Err(_) => match command.locale.as_str() {
                            "ru" => format!("Не удалось подключиться к каналу!"),
                            _ => format!("Could not connect to the channel!")
                        }
                    }
                },
                None => {
                    let (channel, message) = get_voice_channel(&ctx, &command).await;
                    match channel {
                        Some(channel) => {
                            match handler.join(channel).await {
                                Ok(_) => match command.locale.as_str() {
                                    "ru" => format!("Подключился к \"<#{}>\"", channel.get()),
                                    _ => format!("Connected to \"<#{}>\"", channel.get())
                                },
                                Err(_) => match command.locale.as_str() {
                                    "ru" => format!("Не удалось подключиться к каналу!"),
                                    _ => format!("Could not connect to the channel!")
                                }
                            }
                        },
                        None => message.unwrap().into()
                    }

                }
            }
        },
        None => match channel {
            Some(channel) => match manager.join(command.guild_id.unwrap(), channel.id).await {
                Ok(handler) => {
                    let mut handler = handler.lock().await;
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
                    );
                    match command.locale.as_str() {
                        "ru" => format!("Подключился к \"<#{}>\"", channel.id.get()),
                        _ => format!("Connected to \"<#{}>\"", channel.id.get())
                    }
                },
                Err(_) => match command.locale.as_str() {
                    "ru" => format!("Не удалось подключиться к каналу!"),
                    _ => format!("Could not connect to the channel!")
                }
            },
            None => {
                let (channel, message) = get_voice_channel(&ctx, &command).await;
                match channel {
                    Some(channel) => match manager.join(command.guild_id.unwrap(), channel).await {
                        Ok(handler) => {
                            let mut handler = handler.lock().await;
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
                            );
                            match command.locale.as_str() {
                                "ru" => format!("Подключился к \"<#{}>\"", channel.get()),
                                _ => format!("Connected to \"<#{}>\"", channel.get())
                            }
                        },
                        Err(_) => match command.locale.as_str() {
                            "ru" => format!("Не удалось подключиться к каналу!"),
                            _ => format!("Could not connect to the channel!")
                        }
                    },
                    None => message.unwrap().into()
                }
            }
        }
    };

    let data = EditInteractionResponse::new().content(text);
    check_msg(command.edit_response(&ctx.http, data).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("join")
        .description("Join a voice channel")
        .description_localized("ru", "Присоединиться к голосовому каналу")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Channel, "channel", "Channel to join")
                .description_localized("ru", "Канал, к которому присоединиться")
                .channel_types(
                    vec![ChannelType::Voice]
                ).required(false),
        ).dm_permission(false)
}