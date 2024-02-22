use std::collections::HashMap;

use serenity::all::ResolvedValue;
use serenity::builder::{CreateCommand, CreateCommandOption, EditInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::check_msg;

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options: &HashMap<_, _> = &command.data.options().into_iter().map(|param| (param.name, param.value)).collect();
    let password = match options.get("password") {
        Some(ResolvedValue::String(password)) => Some(password),
        _ => None
    }.expect("Cannot find password option");
    check_msg(command.defer_ephemeral(&ctx.http).await);
    let data = EditInteractionResponse::new().content(
        match command.locale.as_str() {
            "ru" => "Установлен новый пароль.",
            _ => "New password is set."
        }
    );
    check_msg(command.edit_response(&ctx.http, data).await);
}

pub fn register() -> CreateCommand {
    CreateCommand::new("password")
        .description("Sets the password in the panel.")
        .description_localized("ru", "Устанавливает пароль в панели.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "password", "The password to set.")
                .description_localized("ru", "Пароль для установки.")
                .min_length(8)
                .max_length(256)
        )
        .dm_permission(true)
}
