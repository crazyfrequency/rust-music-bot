use serenity::{all::{ChannelId, CommandInteraction}, client::Context, builder::{CreateInteractionResponseMessage, CreateInteractionResponse}};

pub mod track;
pub mod playlist;
pub mod player;
pub mod parser;
pub mod gstreamr_input;

pub async fn get_voice_channel(ctx: &Context, command: &CommandInteraction) -> (Option<ChannelId>, Option<impl Into<String>>) {
    match ctx.cache.guild(command.guild_id.unwrap()) {
        Some(guild) => {
            match guild.voice_states.get(&command.user.id) {
                Some(voice_state) => {
                    match voice_state.channel_id {
                        Some(channel_id) => (Some(channel_id), None),
                        None => {
                            let text = match command.locale.as_str() {
                                "ru" => "Подключитесь к каналу!",
                                _ => "Join the channel!"
                            };
                            (None, Some(text))
                        }
                    }
                },
                None => {
                    let text = match command.locale.as_str() {
                        "ru" => "Подключитесь к каналу!",
                        _ => "Join the channel!"
                    };
                    (None, Some(text))
                }
            }
        },
        None => {
            todo!();
            (None, None)
        }
    }
}

use serenity::Result as SerenityResult;

use self::track::Track;
pub fn check_msg<T>(result: SerenityResult<T>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub fn get_title_author_str(track: &Track, locale: &str) -> String {
    let title = match track.title.clone() {
        Some(title) => title,
        None => match locale {
            "ru" => "Название не известно",
            _ => "Unknown title",
        }.to_string()
    };
    let author = match track.author.name.clone() {
        Some(author) => author,
        None => match locale {
            "ru" => "Автор не известен",
            _ => "Unknown author",
        }.to_string()
    };
    format!("`{}` {}", title, author)
}