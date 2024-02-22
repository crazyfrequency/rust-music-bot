use serenity::builder::{CreateCommand, EditInteractionResponse};
use serenity::client::Context;
use serenity::model::application::CommandInteraction;

use crate::bot::utils::check_msg;
use crate::bot::utils::player::clear_guild_player;

pub async fn run(ctx: Context, command: CommandInteraction) {
    check_msg(command.defer_ephemeral(&ctx.http).await);

    clear_guild_player(&ctx, command.guild_id.unwrap()).await;

    let manager = songbird::get(&ctx).await.expect("Songbird Voice client placed in at initialisation.").clone();

    let text = match manager.get(command.guild_id.unwrap()) {
        Some(handler) => {
            let mut handler = handler.lock().await;
            match handler.current_channel() {
                Some(_) => {
                    handler.stop();
                    let res = handler.leave().await;
                    match res {
                        Ok(_) => match command.locale.as_str() {
                            "ru" => "Успешно отключен.",
                            _ => "Successfully disconnected."
                        },
                        Err(_) => match command.locale.as_str() {
                            "ru" => "Не удалось отключиться.",
                            _ => "Could not disconnect."
                        }
                    }
                    
                },
                None => match command.locale.as_str() {
                    "ru" => "Не подключен!",
                    _ => "Not connected!"
                }
            }
        },
        None => match command.locale.as_str() {
            "ru" => "Не подключен!",
            _ => "Not connected!"
        }
    };

    let data = EditInteractionResponse::new().content(text);
    check_msg(command.edit_response(&ctx.http, data).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("disconnect")
        .description("Disconnect from the current voice channel")
        .description_localized("ru", "Отключение от канала")
        .dm_permission(false)
}