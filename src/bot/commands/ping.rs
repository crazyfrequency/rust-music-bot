use serenity::builder::{CreateCommand, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::gateway::ActivityData;
use serenity::model::application::CommandInteraction;

use crate::bot::utils::check_msg;

pub async fn run(ctx: Context, command: CommandInteraction) {
    let data = CreateInteractionResponseMessage::new().content("pong").ephemeral(true);
    let builder = CreateInteractionResponse::Message(data);
    check_msg(command.create_response(&ctx.http, builder).await);
    ctx.shard.clone().set_activity(Some(ActivityData::playing(format!("music, {}", ctx.shard_id.0))));
}

pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("pong")
}
