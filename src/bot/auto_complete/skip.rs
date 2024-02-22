use chrono::Utc;
use serenity::{client::Context, all::CommandInteraction, builder::{CreateInteractionResponse, CreateAutocompleteResponse}};
use strsim::{levenshtein, normalized_damerau_levenshtein};

use crate::bot::utils::{player::PlayerData, check_msg};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let start = Utc::now();
    let track_input = command.data.autocomplete().unwrap();
    
    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();
    let player_playlist = player.playlist.read().await;
    let mut playlist = player_playlist.tracks.clone();
    if let Some(track) = player_playlist.current.clone() {
        playlist.push_front(track);
    }

    if track_input.value.is_empty() {
        let mut choices = CreateAutocompleteResponse::new();
        playlist.iter().take(25)
            .enumerate()
            .for_each(|(index, track)| {
                let title = match track.title.clone() {
                    Some(title) => title,
                    None => match command.locale.as_str() {
                        "ru" => "Название не известно",
                        _ => "Unknown title",
                    }.to_string()
                };
                let author = match track.author.name.clone() {
                    Some(author) => author,
                    None => match command.locale.as_str() {
                        "ru" => "Автор не известен",
                        _ => "Unknown author",
                    }.to_string()
                };
                let mut string = format!("{}: {} - {}", index, title, author);
                if string.len() > 100 {
                    string = string[..100].to_string();
                }
                choices = choices.clone().add_string_choice(string, track.id.to_string());
            });
        let builder = CreateInteractionResponse::Autocomplete(choices);
        check_msg(command.create_response(&ctx.http, builder).await);
        return ;
    }

    let mut distances: Vec<(f64, String, u64)> = playlist
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let title = match track.title.clone() {
                Some(title) => title,
                None => match command.locale.as_str() {
                    "ru" => "Название не известно",
                    _ => "Unknown title",
                }.to_string()
            };
            let author = match track.author.name.clone() {
                Some(author) => author,
                None => match command.locale.as_str() {
                    "ru" => "Автор не известен",
                    _ => "Unknown author",
                }.to_string()
            };
            let string = format!("{}: {} - {}", index, title, author);
            let mut formatted_string = string.clone();
            if string.len() > 100 {
                formatted_string = string[..100].to_string();
            }
            (normalized_damerau_levenshtein(track_input.value.to_lowercase().as_str(), &string.to_lowercase()), formatted_string, track.id)
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