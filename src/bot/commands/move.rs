use std::collections::HashMap;

use serenity::all::ResolvedValue;
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::check_msg;
use crate::bot::utils::parser::{get_time, get_time_str};
use crate::bot::utils::player::{PlayerData, PlayerState, initialize_guild_player, Position};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();
    let to = match options.get("position") {
        Some(ResolvedValue::String(position)) => Some(*position),
        _ => None
    }.expect("position option parse error");
    let to = get_time(to);

    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();

    let mut state = player.state.write().await;
    let player_handler = player.player.write().await.clone();

    let text = match *state {
        PlayerState::Playing | PlayerState::Paused => match player_handler {
            Some(handler) => {
                match handler.stop() {
                    Ok(_) => {
                        let mut position = player.position.write().await;
                        *state = PlayerState::Seeking;
                        *position = Position::from_secs_f64(to);
                        match command.locale.as_str() {
                            "ru" => format!("Перемещено на позицию: {}", get_time_str(to)),
                            _ => format!("Moved to position: {}", get_time_str(to))
                        }
                    },
                    Err(_) => match command.locale.as_str() {
                        "ru" => "Произошла ошибка при перемещении!".to_string(),
                        _ => "An error occurred while moving!".to_string()
                    }
                }
            },
            None => match command.locale.as_str() {
                "ru" => "Не удалось получить плеер!".to_string(),
                _ => "Could not get player!".to_string()
            }
        },
        _ => match command.locale.as_str() {
            "ru" => "Не возможно выполнить перемещение!".to_string(),
            _ => "Cannot perform the move!".to_string()
        }
    };

    let data = CreateInteractionResponseMessage::new().content(text).ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("move")
        .description("Seek to a specific position in the track")
        .description_localized("ru", "Перемещение по треку на заданную минуту/секунду")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "position", "Position to seek( 90 | 1:30 | 0:01:30 )")
                .description_localized("ru", "Позиция для перемещения( 90 | 1:30 | 0:01:30 )")
                .set_autocomplete(true)
                .required(true),
        ).dm_permission(false)
}