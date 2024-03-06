use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context, EditMessage};

use crate::bot::utils::{check_msg, player::{initialize_guild_player, PlayerData}};


pub async fn run(ctx: Context, component: ComponentInteraction) {
    match &component.data.kind {
        ComponentInteractionDataKind::Button => {},
        _ => return
    };

    initialize_guild_player(&ctx, component.guild_id.unwrap()).await;
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&component.guild_id.unwrap().get()).unwrap().clone();
    let settings = player.settings.read().await;

    let builder = EditMessage::new()
        .content(format!("```java\n{}```", settings.equalizer.display()));
    let mut message = component.clone().message;
    check_msg(message.edit(&ctx.http, builder).await);
    check_msg(component.defer(&ctx.http).await);
}