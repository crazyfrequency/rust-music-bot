use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::check_msg;
use crate::bot::utils::player::{PlayerData, PlayerState, initialize_guild_player};

pub async fn run(ctx: Context, command: CommandInteraction) {
    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();

    let mut state = player.state.write().await;
    let player_handler = player.player.write().await.clone();

    let text = match *state {
        PlayerState::Playing | PlayerState::Paused => match player_handler {
            Some(handler) => {
                match handler.pause() {
                    Ok(_) => {
                        *state = PlayerState::Paused;
                        match command.locale.as_str() {
                            "ru" => "Остановлено.",
                            _ => "Paused."
                        }
                    },
                    Err(_) => match command.locale.as_str() {
                        "ru" => "Не удалось остановить!",
                        _ => "Could not pause!"
                    }
                }
            },
            None => match command.locale.as_str() {
                "ru" => "Не удалось остановить!",
                _ => "Could not pause!",
            }
        },
        _ => match command.locale.as_str() {
            "ru" => "Невозможно остановить!",
            _ => "Cannot pause!"
        }
    };

    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("pause")
        .description("Pause the currently playing track")
        .description_localized("ru", "Ставит на паузу текущий трек")
        .dm_permission(false)
}