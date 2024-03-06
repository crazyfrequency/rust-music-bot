use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context, EditMessage};

use crate::bot::utils::{check_msg, player::{initialize_guild_player, Equalizer, PlayerData}};


pub async fn run(ctx: Context, component: ComponentInteraction) {
    let mode = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => Some(values),
        _ => None
    };

    if mode.is_none() {
        check_msg(component.defer(&ctx.http).await);
        return ;
    }
    let mode = mode.unwrap().get(0).unwrap();

    initialize_guild_player(&ctx, component.guild_id.unwrap()).await;
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&component.guild_id.unwrap().get()).unwrap().clone();
    let mut settings = player.settings.write().await;
    let mut ffmpeg = player.ffmpeg.write().await;

    let preset = Equalizer::get_preset(mode);

    let eq = Equalizer {
        f_32: preset[0],
        f_64: preset[1],
        f_125: preset[2],
        f_250: preset[3],
        f_500: preset[4],
        f_1k: preset[5],
        f_2k: preset[6],
        f_4k: preset[7],
        f_8k: preset[8],
        f_16k: preset[9]
    };
    
    settings.set_equalizer(&ctx, eq, ffmpeg.as_mut()).await;

    let builder = EditMessage::new()
        .content(format!("```java\n{}```", settings.equalizer.display()));
    let mut message = component.clone().message;
    check_msg(message.edit(&ctx.http, builder).await);
    check_msg(component.defer(&ctx.http).await);
}