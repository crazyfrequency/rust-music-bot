use std::{sync::Arc, collections::HashMap, process::ChildStdin, io::Write, time::Duration};

use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, result::Error::NotFound, ExpressionMethods, Insertable, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use serenity::{all::GuildId, client::Context, futures::lock::Mutex};
use songbird::{typemap::TypeMapKey, tracks::TrackHandle};
use tokio::sync::RwLock;

use super::playlist::Playlist;
use crate::models::{GuildSettingsDB, UpdateBass};

pub struct PlayerData;

impl TypeMapKey for PlayerData {
    type Value = Arc<RwLock<HashMap<u64, Arc<Player>>>>;
}

pub struct PlayerDataBase;

impl TypeMapKey for PlayerDataBase {
    type Value = Pool<ConnectionManager<SqliteConnection>>;
}

#[derive(Debug)]
pub struct Player {
    pub ffmpeg: Arc<RwLock<Option<ChildStdin>>>,
    pub guild_id: GuildId,
    pub player: Arc<RwLock<Option<TrackHandle>>>,
    pub playlist: Arc<RwLock<Playlist>>,
    pub playlist_sync_and_last_id: Arc<Mutex<u64>>,
    pub settings: Arc<RwLock<PlayerSettings>>,
    pub position: Arc<RwLock<Position>>,
    pub state: Arc<RwLock<PlayerState>>

}

impl Player {
    pub async fn new(ctx: &Context, guild_id: GuildId) -> Self {
        Self {
            ffmpeg: Arc::new(RwLock::new(None)),
            guild_id,
            player: Arc::new(RwLock::new(None)),
            playlist: Arc::new(RwLock::new(Playlist::new())),
            playlist_sync_and_last_id: Arc::new(Mutex::new(0)),
            settings:
                Arc::new(RwLock::new(
                    PlayerSettings::new(ctx, guild_id.get()).await
                )),
            position: Arc::new(RwLock::new(Position::default())),
            state: Arc::new(RwLock::new(PlayerState::Ended)),
        }
    }

    pub async fn new_with_pool<T>(pool: Result<PooledConnection<ConnectionManager<SqliteConnection>>, T>, guild_id: GuildId) -> Self {
        Self {
            ffmpeg: Arc::new(RwLock::new(None)),
            guild_id,
            player: Arc::new(RwLock::new(None)),
            playlist: Arc::new(RwLock::new(Playlist::new())),
            playlist_sync_and_last_id: Arc::new(Mutex::new(0)),
            settings:
                Arc::new(RwLock::new(
                    PlayerSettings::new_with_pool(pool, guild_id.get()).await
                )),
            position: Arc::new(RwLock::new(Position::default())),
            state: Arc::new(RwLock::new(PlayerState::Ended)),
        }
    }

    pub async fn clear(&self) {
        *self.playlist.write().await = Playlist::new();
        *self.state.write().await = PlayerState::Ended;
        *self.position.write().await = Position::default();
        *self.ffmpeg.write().await = None;
        let mut player_handler = self.player.write().await;
        if let Some(player_handler) = player_handler.as_mut() {
            let _ = player_handler.stop();
        }
        *player_handler = None;
    }
}

#[derive(Debug)]
pub struct Position {
    pub last_position: Duration,
    pub last_player_position: Duration
}

impl Position {
    pub fn default() -> Self {
        Position {
            last_position: Duration::from_secs(0),
            last_player_position: Duration::from_secs(0)
        }
    }

    pub fn from_secs(secs: u64) -> Self {
        Position {
            last_position: Duration::from_secs(secs),
            last_player_position: Duration::from_secs(0)
        }
    }

    pub fn from_secs_f64(secs: f64) -> Self {
        Position {
            last_position: Duration::from_secs_f64(secs),
            last_player_position: Duration::from_secs(0)
        }
    }
}

#[derive(Debug)]
pub struct PlayerSettings {
    guild_id: u64,
    pub speed: f64,
    pub volume: f64,
    pub bass_enabled: bool,
    pub bass_gain: f64,
    pub equalizer: Equalizer,
    pub repeat: RepeatMode
}

impl PlayerSettings {
    pub async fn new(ctx: &Context, guild_id: u64) -> Self {
        let pool = {
            let data_read = ctx.data.read().await;
            let conn = data_read.get::<PlayerDataBase>().expect("Expected PlayerDataBase in TypeMap.").clone();
            conn.get()
        };
        Self::new_with_pool(pool, guild_id).await
    }

    pub async fn new_with_pool<T>(pool: Result<PooledConnection<ConnectionManager<SqliteConnection>>, T>, guild_id: u64) -> Self {
        match pool {
            Ok(mut pool) => {
                use crate::schema::guild_settings::dsl::*;
                match guild_settings.find(guild_id as i64).first::<GuildSettingsDB>(&mut pool) {
                    Ok(settings) => {
                        let settings = settings.clone();
                        settings as GuildSettingsDB;
                        println!("{:?}", settings);
                        return PlayerSettings {
                            guild_id,
                            speed: settings.speed,
                            volume: settings.volume,
                            bass_enabled: settings.bass_enabled,
                            bass_gain: settings.bass_gain,
                            equalizer: Equalizer {
                                f_32: settings.equalizer_32,
                                f_64: settings.equalizer_64,
                                f_125: settings.equalizer_125,
                                f_250: settings.equalizer_250,
                                f_500: settings.equalizer_500,
                                f_1k: settings.equalizer_1k,
                                f_2k: settings.equalizer_2k,
                                f_4k: settings.equalizer_4k,
                                f_8k: settings.equalizer_8k,
                                f_16k: settings.equalizer_16k
                            },
                            repeat: RepeatMode::new(settings.loop_type)
                        };
                    },
                    Err(NotFound) => {
                        let _ = GuildSettingsDB::new(guild_id).insert_into(guild_settings).execute(&mut pool);
                    },
                    _ => {}
                };
            },
            Err(_) => {}
        };
        PlayerSettings {
            guild_id,
            speed: 1.0,
            volume: 1.0,
            bass_enabled: false,
            bass_gain: 20.0,
            equalizer: Equalizer {
                f_32: 0.0,
                f_64: 0.0,
                f_125: 0.0,
                f_250: 0.0,
                f_500: 0.0,
                f_1k: 0.0,
                f_2k: 0.0,
                f_4k: 0.0,
                f_8k: 0.0,
                f_16k: 0.0
            },
            repeat: RepeatMode::Off
        }
    }

    pub async fn set_repeat(&mut self, ctx: &Context, repeat: RepeatMode) {
        let pool = {
            let data_read = ctx.data.read().await;
            let conn = data_read.get::<PlayerDataBase>().expect("Expected PlayerDataBase in TypeMap.").clone();
            conn.get()
        };
        match pool {
            Ok(mut pool) => {
                use crate::schema::guild_settings::dsl::*;
                let _ = diesel::update(guild_settings.filter(id.eq(self.guild_id as i64))).set(loop_type.eq(repeat as i16)).execute(&mut pool);
            },
            Err(_) => {}
        };
        self.repeat = repeat;
    }

    pub async fn set_volume(&mut self, ctx: &Context, volume_value: f64, ffmpeg: Option<&mut ChildStdin>) {
        let pool = {
            let data_read = ctx.data.read().await;
            let conn = data_read.get::<PlayerDataBase>().expect("Expected PlayerDataBase in TypeMap.").clone();
            conn.get()
        };
        match pool {
            Ok(mut pool) => {
                use crate::schema::guild_settings::dsl::*;
                let _ = diesel::update(guild_settings
                    .filter(id.eq(self.guild_id as i64)))
                    .set(volume.eq(volume_value))
                    .execute(&mut pool);
            },
            Err(_) => {}
        };
        self.volume = volume_value;
        if let Some(ffmpeg) = ffmpeg {
            let _ = ffmpeg.write(format!("^Cvolume -1 volume {}\n", self.volume * 0.2).as_bytes());
        }
    }
    
    pub async fn set_speed(&mut self, ctx: &Context, speed_value: f64, ffmpeg: Option<&mut ChildStdin>) {
        let pool = {
            let data_read = ctx.data.read().await;
            let conn = data_read.get::<PlayerDataBase>().expect("Expected PlayerDataBase in TypeMap.").clone();
            conn.get()
        };
        match pool {
            Ok(mut pool) => {
                use crate::schema::guild_settings::dsl::*;
                let res = diesel::update(guild_settings
                    .filter(id.eq(self.guild_id as i64)))
                    .set(speed.eq(speed_value))
                    .execute(&mut pool);
                println!("{:?}", res);
            }
            Err(_) => {}
        };
        self.speed = speed_value;
        if let Some(ffmpeg) = ffmpeg {
            let _ = ffmpeg.write(format!("^Catempo -1 tempo {}\n", self.speed).as_bytes());
        }
    }

    pub async fn set_bass(&mut self, ctx: &Context, bass_on: Option<bool>, bass_value: Option<f64>, ffmpeg: Option<&mut ChildStdin>) {
        let pool = {
            let data_read = ctx.data.read().await;
            let conn = data_read.get::<PlayerDataBase>().expect("Expected PlayerDataBase in TypeMap.").clone();
            conn.get()
        };
        match pool {
            Ok(mut pool) => {
                use crate::schema::guild_settings::dsl::*;
                let res = diesel::update(guild_settings
                    .filter(id.eq(self.guild_id as i64)))
                    .set(UpdateBass {
                        bass_enabled: bass_on.unwrap_or(self.bass_enabled),
                        bass_gain: bass_value.unwrap_or(self.bass_gain)
                    })
                    .execute(&mut pool);
                println!("{:?}", res);
            }
            Err(_) => {}
        };
        if let Some(bass_on) = bass_on {
            self.bass_enabled = bass_on;
        }
        if let Some(bass_value) = bass_value {
            self.bass_gain = bass_value;
        }
        if let Some(ffmpeg) = ffmpeg {
            let _ = match self.bass_enabled {
                true => ffmpeg.write(format!("^Cbass -1 g {}\n", self.bass_gain).as_bytes()),
                false => ffmpeg.write("^Cbass -1 g 0\n".as_bytes()),
            };
        }
    }
}

#[derive(Debug,PartialEq)]
pub struct Equalizer {
    pub f_32: f64,
    pub f_64: f64,
    pub f_125: f64,
    pub f_250: f64,
    pub f_500: f64,
    pub f_1k: f64,
    pub f_2k: f64,
    pub f_4k: f64,
    pub f_8k: f64,
    pub f_16k: f64
}

#[derive(Debug)]
pub enum PlayerState {
    Ended,
    Starting,
    Playing,
    Paused,
    InSkip,
    Seeking
}

#[derive(Debug,PartialEq,Clone,Copy)]
pub enum RepeatMode {
    Off,
    Track,
    Queue
}

impl RepeatMode {
    pub fn new(mode: i16) -> Self {
        match mode {
            1 => Self::Track,
            2 => Self::Queue,
            _ => Self::Off
        }
    }
}

pub type PlayerDataType = Arc<RwLock<HashMap<u64, Arc<Player>>>>;

pub async fn initialize_guild_player(ctx: &Context, guild_id: GuildId) {
    let map = {
        let data_read = ctx.data.read().await;
        data_read.get::<PlayerData>().expect("Expected PlayerData in TypeMap.").clone()
    };
    let mut map = map.write().await;

    match map.get(&guild_id.get()) {
        Some(_) => {},
        None => {
            let player =  Arc::new(Player::new(&ctx, guild_id).await);
            map.insert(guild_id.get(), player);
        }
    };
}

pub async fn initialize_guild_player_web<T>(player: &PlayerDataType, pool: Result<PooledConnection<ConnectionManager<SqliteConnection>>, T> , guild_id: GuildId) {
    let mut player = player.write().await;

    match player.get(&guild_id.get()) {
        Some(_) => {},
        None => {
            let new_player =  Arc::new(Player::new_with_pool(pool, guild_id).await);
            player.insert(guild_id.get(), new_player);
        }
    };
}

pub async fn clear_guild_player(ctx: &Context, guild_id: GuildId) {
    let map = {
        let data_read = ctx.data.read().await;
        data_read.get::<PlayerData>().expect("Expected PlayerData in TypeMap.").clone()
    };
    let mut map = map.write().await;

    match map.get(&guild_id.get()) {
        Some(player) => {
            player.clear().await;
        },
        None => {
            let player =  Arc::new(Player::new(&ctx, guild_id).await);
            map.insert(guild_id.get(), player);
        }
    };
}