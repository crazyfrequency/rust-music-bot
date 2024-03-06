use std::collections::HashMap;
use std::time::Duration;

use serenity::all::{ButtonStyle, Color, CreateActionRow, CreateButton, CreateEmbed, CreateSelectMenu, CreateSelectMenuKind, ResolvedOption, ResolvedValue};
use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponseMessage, CreateInteractionResponse};
use serenity::client::Context;
use serenity::model::application::{CommandOptionType, CommandInteraction};

use crate::bot::utils::check_msg;
use crate::bot::utils::player::{initialize_guild_player, Equalizer, PlayerData, SetEqualizer};

pub async fn run(ctx: Context, command: CommandInteraction) {
    let options = command.data.options();
    let options = options.get(0);

    let (name, options) = match options {
        Some(ResolvedOption { name, value, .. }) => match value {
            ResolvedValue::SubCommand(command) => (*name, command),
            _ => return
        },
        _ => return
    };

    initialize_guild_player(&ctx, command.guild_id.unwrap()).await;

    let player = ctx.data.read().await.get::<PlayerData>().unwrap().clone();
    let player = player.read().await.clone();
    let player = player.get(&command.guild_id.unwrap().get()).unwrap().clone();

    if name == "get" {
        let settings = player.settings.read().await;
        let presets = Equalizer::presets(&command.locale);
        let (first_presets, second_presets) = presets.split_at(25);

        let data = CreateInteractionResponseMessage::new()
            .content(format!("```java\n{}```", settings.equalizer.display()))
            .embed(
                CreateEmbed::new()
                    .color(Color::DARK_PURPLE)
                    .title(
                        match command.locale.as_str() {
                            "ru" => "Параметры эквалайзера",
                            _ => "Equalizer parameters",
                        }
                    ).fields(
                        settings.equalizer.to_fields(&command.locale)
                    )
            ).components(
                vec![
                    CreateActionRow::Buttons(
                        vec![
                            CreateButton::new(
                                "eq_update"
                            ).label(match command.locale.as_str() {
                                "ru" => "Обновить",
                                _ => "Update",
                            })
                        ]
                    ),
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(
                            "eq_presets1",
                            CreateSelectMenuKind::String{
                                options: first_presets.to_vec()
                            }
                        ).placeholder(match command.locale.as_str() {
                            "ru" => "Пресеты 1",
                            _ => "Presets 1",
                        })
                    ),
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(
                            "eq_presets2",
                            CreateSelectMenuKind::String{
                                options: second_presets.to_vec()
                            }
                        ).placeholder(match command.locale.as_str() {
                            "ru" => "Пресеты 2",
                            _ => "Presets 2",
                        })
                    )
                ]
            );
        let builder = CreateInteractionResponse::Message(data);
        check_msg(command.create_response(&ctx.http, builder).await);
    } else if name == "set" {
        let options: &HashMap<_, _> = &options.into_iter().map(|param| (param.name, param.clone().value)).collect();

        println!("{:?}", options);

        let frequency = match options.get("frequency") {
            Some(ResolvedValue::Integer(f)) => f,
            _ => return
        };

        let gain = match options.get("gain") {
            Some(ResolvedValue::Number(v)) => v,
            _ => return
        };

        let mut settings = player.settings.write().await;
        let mut ffmpeg = player.ffmpeg.write().await;
        
        match frequency {
            0..=9 => settings.set_equalizer(&ctx, SetEqualizer::new(*frequency, *gain), ffmpeg.as_mut()).await,
            _ => {}
        };

        let data = CreateInteractionResponseMessage::new()
            .content("ok")
            .ephemeral(true);
        let builder = CreateInteractionResponse::Message(data);
        check_msg(command.create_response(&ctx.http, builder).await);
    }
}

pub fn register() -> CreateCommand {
    CreateCommand::new("equalizer")
        .description("Setting up the equalizer")
        .description_localized("ru", "Настройка эквалайзера")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "get", "Get equalizer parameters")
                .name_localized("ru", "получить")
                .description_localized("ru", "Получить параметры эквалайзера")
        ).add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "set", "Set equalizer parameter")
                .name_localized("ru", "установить")
                .description_localized("ru", "Установить параметры эквалайзера")
                .add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Integer, "frequency", "Frequency")
                        .name_localized("ru", "частота")
                        .description_localized("ru", "Частота")
                        .add_int_choice_localized("32hz", 0, [("ru", "32Гц")])
                        .add_int_choice_localized("64hz", 1, [("ru", "64Гц")])
                        .add_int_choice_localized("125hz", 2, [("ru", "125Гц")])
                        .add_int_choice_localized("250hz", 3, [("ru", "250Гц")])
                        .add_int_choice_localized("500hz", 4, [("ru", "500Гц")])
                        .add_int_choice_localized("1khz", 5, [("ru", "1кГц")])
                        .add_int_choice_localized("2khz", 6, [("ru", "2кГц")])
                        .add_int_choice_localized("4khz", 7, [("ru", "4кГц")])
                        .add_int_choice_localized("8khz", 8, [("ru", "8кГц")])
                        .add_int_choice_localized("16khz", 9, [("ru", "16кГц")])
                        .required(true)
                ).add_sub_option(
                    CreateCommandOption::new(CommandOptionType::Number, "gain", "Gain(db)")
                        .name_localized("ru", "усиление")
                        .description_localized("ru", "Усиление(дб)")
                        .min_number_value(-50.0)
                        .max_number_value(50.0)
                        .required(true)
                )
        )
        .dm_permission(false)
}
