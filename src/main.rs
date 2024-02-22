use std::{collections::HashMap, env, sync::Arc};

use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use bot::{commands, utils::player::{PlayerData, PlayerDataType, PlayerDataBase}, auto_complete};
use diesel::{r2d2::ConnectionManager, SqliteConnection};
use serenity::{
    all::Command, async_trait, client::Cache, model::{gateway::Ready, application::Interaction}, prelude::*
};
use songbird::{SerenityInit, Songbird, Config};

mod api;
mod bot;
mod models;
mod schema;

struct DiscordClient;

#[async_trait]
impl EventHandler for DiscordClient {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => match command.data.name.as_str() {
                "ping" => commands::ping::run(ctx, command).await,
                "play" => commands::play::run(ctx, command).await,
                "volume" => commands::volume::run(ctx, command).await,
                "pause" => commands::pause::run(ctx, command).await,
                "resume" => commands::resume::run(ctx, command).await,
                "skip" => commands::skip::run(ctx, command).await,
                "move" => commands::r#move::run(ctx, command).await,
                "disconnect" => commands::disconnect::run(ctx, command).await,
                "join" => commands::join::run(ctx, command).await,
                "repeat" => commands::repeat::run(ctx, command).await,
                "speed" => commands::speed::run(ctx, command).await,
                "password" => commands::password::run(ctx, command).await,
                "bass" => commands::bass::run(ctx, command).await,
                _ => {}
            },
            Interaction::Autocomplete(autocomplete) => match autocomplete.data.name.as_str() {
                "skip" => auto_complete::skip::run(ctx, autocomplete).await,
                "move" => auto_complete::r#move::run(ctx, autocomplete).await,
                _ => {}
            }
            _ => {}
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Bot {} is connected!", ready.user.name);

        Command::set_global_commands(&ctx.http, vec![
            commands::play::register(),
            commands::ping::register(),
            commands::volume::register(),
            commands::pause::register(),
            commands::resume::register(),
            commands::skip::register(),
            commands::r#move::register(),
            commands::disconnect::register(),
            commands::join::register(),
            commands::repeat::register(),
            commands::speed::register(),
            commands::password::register(),
            commands::bass::register(),
        ]).await.expect("commands load error");
    }
}

async fn index() -> impl Responder {
    // let options = SearchOptions::youtube("fat rat end of decade").with_count(1);
    let data = youtube_dl::YoutubeDl::new("https://www.youtube.com/watch?v=whFmuLRRPKU&list=PL37UZ2QfPUvz3k5mZNFZmjhLNTT-M5-Oa").cookies("./cookies.txt").flat_playlist(true).socket_timeout("15").run_raw_async().await.unwrap();
    // let track: Result<Playlist, serde_json::Error> = serde_json::from_value(data.clone());
    HttpResponse::Ok().json(data)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Warn)
        .init();

    let player_data: PlayerDataType = Arc::new(RwLock::new(HashMap::default()));
    let player_data_clone = player_data.clone();

    let db = ConnectionManager::<SqliteConnection>::new("sqlite://db.sqlite3");
    let pool = diesel::r2d2::Pool::builder()
        .build(db)
        .expect("Failed to build pool manager");
    let pool_clone = pool.clone();

    let songbird = Songbird::serenity();
    songbird.set_config(Config::default().preallocated_tracks(2));
    let songbird_clone = songbird.clone();

    let token = env::var("BOT_TOKEN").expect("Expected a token in the environment");
    let mut client = Client::builder(token, GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES)
        .event_handler(DiscordClient)
        .register_songbird_with(songbird_clone)
        .await
        .expect("Error creating client");

    let cache_clone = client.cache.clone();
    
    // Запуск веб-сервера Actix
    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            // .wrap(
            //     middleware::DefaultHeaders::new().add((header::X_CONTENT_TYPE_OPTIONS, "nosniff")),
            // )
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .wrap(actix_cors::Cors::permissive())
            .service(web::resource("/").to(index))
            .service(api::controllers::api_scope())
            .app_data(web::Data::new(player_data.clone()))
            .app_data(web::Data::new(songbird.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(None::<Arc<Cache>>))
            .app_data(web::Data::new(cache_clone.clone()))
    })
    .bind("127.0.0.1:8081")?;

    // Запуск Discord-клиента в рамках той же runtime (tokio)
    tokio::spawn(async move {

        {
            // Open the data lock in write mode, so keys can be inserted to it.
            let mut data = client.data.write().await;

            data.insert::<PlayerData>(player_data_clone);
            data.insert::<PlayerDataBase>(pool_clone);
        }

        if let Err(why) = client.start_shards(2).await {
            println!("An error occurred while running the client: {:?}", why);
        }
    });
    server.run().await
}

