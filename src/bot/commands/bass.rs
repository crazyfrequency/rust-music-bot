use std::collections::HashMap;
use std::time::Duration;

use serenity::all::ResolvedValue;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::check_msg;
use crate::bot::utils::player::{initialize_guild_player, PlayerData};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();

    let enable = match options.get("enable") {
        Some(ResolvedValue::Boolean(enable)) => Some(*enable),
        _ => None
    };

    let db = match options.get("db") {
        Some(ResolvedValue::Number(db)) => Some(*db),
        _ => None
    };

    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;

    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();
    let mut settings = player.settings.write().await;
    let mut ffmpeg = player.ffmpeg.write().await;

    if db.is_some() || enable.is_some() {
        settings.set_bass(&ctx ,enable, db, ffmpeg.as_mut()).await;
    }

    let text = match command.locale.as_str() {
        "ru" => format!("Бас `{}`, значение установленно как {}.", 
            if settings.bass_enabled { "включен" } else { "выключен" },
            settings.bass_gain
        ),
        _ => format!("Bass `{}`, set to {}.", 
            if settings.bass_enabled { "enabled" } else { "disabled" },
            settings.bass_gain
        )
    };
    
    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("bass")
        .description("Setting up the bass mode")
        .description_localized("ru", "Настройка режима баса")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Boolean, "enable", "Enabling/disabling the bass mode")
                .description_localized("ru", "Включение/выключение режима баса")
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Number, "db", "Setting the value of the bass in dB(0 without changes, -10 on 10dB less)")
                .description_localized("ru", "Значение баса в дБ(0 без изменений, -10 на 10дБ тише)")
                .min_number_value(-100.0)
                .max_number_value(100.0)
        ).dm_permission(false)
}
