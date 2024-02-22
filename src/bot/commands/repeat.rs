use std::collections::HashMap;

use serenity::all::ResolvedValue;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::check_msg;
use crate::bot::utils::player::{PlayerData, PlayerState, initialize_guild_player, RepeatMode};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();
    let mode = match options.get("mode") {
        Some(ResolvedValue::Integer(mode)) => Some(*mode),
        _ => None
    };
    
    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();

    let mut settings = player.settings.write().await;

    let text = match mode {
        Some(0) => {
            settings.set_repeat(&ctx, RepeatMode::Off).await;
            match command.locale.as_str() {
                "ru" => "Режим повтора отключен.",
                _ => "Repeat mode is disabled."
            }
        },
        Some(1) => {
            settings.set_repeat(&ctx, RepeatMode::Track).await;
            match command.locale.as_str() {
                "ru" => "Установлен режим повтора одного трека.",
                _ => "Set repeat mode to one track."
            }
        },
        Some(2) => {
            settings.set_repeat(&ctx, RepeatMode::Queue).await;
            match command.locale.as_str() {
                "ru" => "Установлен режим повтора всех треков.",
                _ => "Set repeat mode to all tracks."
                
            }
        },
        _ => match settings.repeat {
            RepeatMode::Off => match command.locale.as_str() {
                "ru" => "Режим повтора отключен.",
                _ => "Repeat mode is disabled."
            },
            RepeatMode::Track => match command.locale.as_str() {
                "ru" => "Включен режим повтора одного трека.",
                _ => "Repeat mode is enabled for one track."
            },
            RepeatMode::Queue => match command.locale.as_str() {
                "ru" => "Включен режим повтора всех треков.",
                _ => "Repeat mode is enabled for all tracks."
            }
        }
    };

    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("repeat")
        .description("Enables/disables track/playlist repeat")
        .description_localized("ru", "Включение/выключение повтора трека/плейлиста")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "mode", "Repeat mode")
                .description_localized("ru", "Режим повтора")
                .add_int_choice_localized("disable", 0, [("ru", "выкл")])
                .add_int_choice_localized("track", 1, [("ru", "трек")])
                .add_int_choice_localized("playlist", 2, [("ru", "плейлист")])
        ).dm_permission(false)
}