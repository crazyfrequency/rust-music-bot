use chrono::Utc;
use serenity::{client::Context, all::CommandInteraction, builder::{CreateAutocompleteResponse, CreateInteractionResponse}};
use strsim::normalized_damerau_levenshtein;

use crate::bot::utils::{player::PlayerData, check_msg};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let start = Utc::now();
    let position = command.data.autocomplete().unwrap();
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();
    let player_playlist = player.playlist.read().await;
    let track = player_playlist.current.clone().unwrap();

    if position.value.is_empty() {
        let mut choices = CreateAutocompleteResponse::new();
        track.chapters.iter().take(25)
            .for_each(|chapter| {
                let mut string = format!("{}: {}", chapter.start_time_str, chapter.title);
                if string.len() > 100 {
                    string = string[..100].to_string();
                }
                choices = choices.clone().add_string_choice(string, chapter.start_time.to_string());
            });
        let builder = CreateInteractionResponse::Autocomplete(choices);
        check_msg(command.create_response(&ctx.http, builder).await);
        return ;
    }

    let mut distances: Vec<(f64, String, f64)> = track.chapters
        .iter().map(|chapter| {
            let string = format!("{}: {}", chapter.start_time_str, chapter.title);
            let mut formatted_string = string.clone();
            if string.len() > 100 {
                formatted_string = string[..100].to_string();
            }
            (normalized_damerau_levenshtein(position.value.to_lowercase().as_str(), &string.to_lowercase()), formatted_string, chapter.start_time)
        })
        .collect();
    distances.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut choices = CreateAutocompleteResponse::new();
    distances.into_iter().take(25).for_each(|(_, text, value)| {
        choices = choices.clone().add_string_choice(text, value.to_string());
    });
    let builder = CreateInteractionResponse::Autocomplete(choices);
    check_msg(command.create_response(&ctx.http, builder).await);
    println!("{}", Utc::now()-start);
}