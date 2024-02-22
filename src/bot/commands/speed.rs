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
    let value = match options.get("value") {
        Some(ResolvedValue::Number(value)) => Some(*value),
        _ => None
    }.expect("url option parse error");

    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;

    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();
    let mut settings = player.settings.write().await;
    let player_handler = player.player.read().await.clone();
    let mut position = player.position.write().await;
    let mut ffmpeg = player.ffmpeg.write().await;

    let speed = settings.speed;
    settings.set_speed(&ctx ,value, ffmpeg.as_mut()).await;

    match player_handler {
        Some(handler) => {
            match handler.get_info().await {
                Ok(data) => {
                    let new_position = data.position;
                    position.last_position = position.last_position + Duration::from_secs_f64((new_position - position.last_player_position).as_secs_f64() * speed + 0.2);
                    position.last_player_position = new_position;
                },
                _ => {}
            }
        },
        None => {}
    }
    println!("{:?}", position);

    let text = match command.locale.as_str() {
        "ru" => format!("Скорость установлена на `{}`.", value),
        _ => format!("Speed set to `{}`.", value)
    };
    
    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("speed")
        .description("Sets the playback speed")
        .description_localized("ru", "Устанавливает скорость воспроизведения")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Number, "value", "The value to set the speed to")
                .description_localized("ru", "Значение, которое нужно установить")
                .min_number_value(0.5)
                .max_number_value(2.0)
                .required(true),
        ).dm_permission(false)
}
