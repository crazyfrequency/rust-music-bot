use std::collections::HashMap;

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
    let mut ffmpeg = player.ffmpeg.write().await;

    settings.set_volume(&ctx ,value, ffmpeg.as_mut()).await;

    let text = match command.locale.as_str() {
        "ru" => format!("Громкость установлена на `{}`.", value),
        _ => format!("Volume set to `{}`.", value)
    };
    
    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("volume")
        .description("Sets the volume of the player")
        .description_localized("ru", "Устанавливает громкость плеера")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Number, "value", "The volume to set")
                .description_localized("ru", "Громкость")
                .required(true),
        ).dm_permission(false)
}
