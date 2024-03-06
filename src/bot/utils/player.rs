use std::{sync::Arc, collections::HashMap, process::ChildStdin, io::Write, time::Duration};

use diesel::{r2d2::{ConnectionManager, Pool, PooledConnection}, result::Error::NotFound, ExpressionMethods, Insertable, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use serenity::{all::{CreateSelectMenuOption, GuildId}, client::Context, futures::lock::Mutex};
use songbird::{typemap::TypeMapKey, tracks::TrackHandle};
use tokio::sync::RwLock;

use super::playlist::Playlist;
use crate::models::{GuildSettingsDB, UpdateBass, UpdateEqualizer};

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

    pub async fn set_equalizer(&mut self, ctx: &Context, equalizer: impl Into<SetEqualizer>, ffmpeg: Option<&mut ChildStdin>) {
        let equalizer = equalizer.into() as SetEqualizer;
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
                    .set(UpdateEqualizer {
                        equalizer_32: equalizer.f_32.unwrap_or(self.equalizer.f_32),
                        equalizer_64: equalizer.f_64.unwrap_or(self.equalizer.f_64),
                        equalizer_125: equalizer.f_125.unwrap_or(self.equalizer.f_125),
                        equalizer_250: equalizer.f_250.unwrap_or(self.equalizer.f_250),
                        equalizer_500: equalizer.f_500.unwrap_or(self.equalizer.f_500),
                        equalizer_1k: equalizer.f_1k.unwrap_or(self.equalizer.f_1k),
                        equalizer_2k: equalizer.f_2k.unwrap_or(self.equalizer.f_2k),
                        equalizer_4k: equalizer.f_4k.unwrap_or(self.equalizer.f_4k),
                        equalizer_8k: equalizer.f_8k.unwrap_or(self.equalizer.f_8k),
                        equalizer_16k: equalizer.f_16k.unwrap_or(self.equalizer.f_16k),
                    })
                    .execute(&mut pool);
                println!("{:?}", res);
            }
            Err(_) => {}
        };
        if let Some(value) = equalizer.f_32 {
            self.equalizer.f_32 = value;
        } if let Some(value) = equalizer.f_64 {
            self.equalizer.f_64 = value;
        } if let Some(value) = equalizer.f_125 {
            self.equalizer.f_125 = value;
        } if let Some(value) = equalizer.f_250 {
            self.equalizer.f_250 = value;
        } if let Some(value) = equalizer.f_500 {
            self.equalizer.f_500 = value;
        } if let Some(value) = equalizer.f_1k {
            self.equalizer.f_1k = value;
        } if let Some(value) = equalizer.f_2k {
            self.equalizer.f_2k = value;
        } if let Some(value) = equalizer.f_4k {
            self.equalizer.f_4k = value;
        } if let Some(value) = equalizer.f_8k {
            self.equalizer.f_8k = value;
        } if let Some(value) = equalizer.f_16k {
            self.equalizer.f_16k = value;
        }
        if let Some(ffmpeg) = ffmpeg {
            if let Some(value) = equalizer.f_32 {
                let _ = ffmpeg.write(format!("^Cequalizer@h32 -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_64 {
                let _ = ffmpeg.write(format!("^Cequalizer@h64 -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_125 {
                let _ = ffmpeg.write(format!("^Cequalizer@h125 -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_250 {
                let _ = ffmpeg.write(format!("^Cequalizer@h250 -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_500 {
                let _ = ffmpeg.write(format!("^Cequalizer@h500 -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_1k {
                let _ = ffmpeg.write(format!("^Cequalizer@h1k -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_2k {
                let _ = ffmpeg.write(format!("^Cequalizer@h2k -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_4k {
                let _ = ffmpeg.write(format!("^Cequalizer@h4k -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_8k {
                let _ = ffmpeg.write(format!("^Cequalizer@h8k -1 g {}\n", value).as_bytes());
            } if let Some(value) = equalizer.f_16k {
                let _ = ffmpeg.write(format!("^Cequalizer@h16k -1 g {}\n", value).as_bytes());
            }
        }
    }
}

pub struct SetEqualizer {
    pub f_32: Option<f64>,
    pub f_64: Option<f64>,
    pub f_125: Option<f64>,
    pub f_250: Option<f64>,
    pub f_500: Option<f64>,
    pub f_1k: Option<f64>,
    pub f_2k: Option<f64>,
    pub f_4k: Option<f64>,
    pub f_8k: Option<f64>,
    pub f_16k: Option<f64>
}

impl SetEqualizer {
    pub fn new(frequency: i64, gain: f64) -> Self {
        Self {
            f_32: if frequency == 0 { Some(gain) } else { None },
            f_64: if frequency == 1 { Some(gain) } else { None },
            f_125: if frequency == 2 { Some(gain) } else { None },
            f_250: if frequency == 3 { Some(gain) } else { None },
            f_500: if frequency == 4 { Some(gain) } else { None },
            f_1k: if frequency == 5 { Some(gain) } else { None },
            f_2k: if frequency == 6 { Some(gain) } else { None },
            f_4k: if frequency == 7 { Some(gain) } else { None },
            f_8k: if frequency == 8 { Some(gain) } else { None },
            f_16k: if frequency == 9 { Some(gain) } else { None }
        }
    }
}

impl From<Equalizer> for SetEqualizer {
    fn from(val: Equalizer) -> Self {
        Self {
            f_32: Some(val.f_32),
            f_64: Some(val.f_64),
            f_125: Some(val.f_125),
            f_250: Some(val.f_250),
            f_500: Some(val.f_500),
            f_1k: Some(val.f_1k),
            f_2k: Some(val.f_2k),
            f_4k: Some(val.f_4k),
            f_8k: Some(val.f_8k),
            f_16k: Some(val.f_16k)
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

impl Equalizer {
    pub fn to_fields(&self, local: impl Into<String>) -> Vec<(&str, String, bool)> {
        match local.into().as_str() {
            "ru" => vec![
                ("32 Гц", format!("{} дб", &self.f_32), true),
                ("64 Гц", format!("{} дб", &self.f_64), true),
                ("125 Гц", format!("{} дб", &self.f_125), true),
                ("250 Гц", format!("{} дб", &self.f_250), true),
                ("500 Гц", format!("{} дб", &self.f_500), true),
                ("1 кГц", format!("{} дб", &self.f_1k), true),
                ("2 кГц", format!("{} дб", &self.f_2k), true),
                ("4 кГц", format!("{} дб", &self.f_4k), true),
                ("8 кГц", format!("{} дб", &self.f_8k), true),
                ("16 кГц", format!("{} дб", &self.f_16k), true)
            ],
            _ => vec![
                ("32 Hz", format!("{} dB", &self.f_32), true),
                ("64 Hz", format!("{} dB", &self.f_64), true),
                ("125 Hz", format!("{} dB", &self.f_125), true),
                ("250 Hz", format!("{} dB", &self.f_250), true),
                ("500 Hz", format!("{} dB", &self.f_500), true),
                ("1 kHz", format!("{} dB", &self.f_1k), true),
                ("2 kHz", format!("{} dB", &self.f_2k), true),
                ("4 kHz", format!("{} dB", &self.f_4k), true),
                ("8 kHz", format!("{} dB", &self.f_8k), true),
                ("16 kHz", format!("{} dB", &self.f_16k), true)
            ]
        }
    }

    pub fn display(&self) -> String {
        let frequencies = [
            self.f_32, self.f_64, self.f_125, self.f_250, self.f_500,
            self.f_1k, self.f_2k, self.f_4k, self.f_8k, self.f_16k,
        ];

        let max_value = 12.0;
        let min_value = -12.0;
        let step = 2.0;

        let mut result = String::new();

        // Добавим линию для значений выше диапазона
        result.push_str("     ");
        for &value in &frequencies {
            result.push_str(if value > max_value { "┯" } else { "╷" });
            result.push_str("   ");
        }
        result.push('\n');

        for level in (min_value as i32..=max_value as i32).rev().step_by(step as usize) {
            let mut line = format!("{:4} ", level);
            for &value in &frequencies {
                let symbol = if (value as i32) == level { "┿" }
                else if value > 0.0 && (value as i32) == level-1 { "┿" }
                else if value < 0.0 && (value as i32) == level+1 { "┿" }
                else { "│" };
                line.push_str(symbol);
                line.push_str("   ");
            }
            line.push('\n');
            result.push_str(&line);
        }

        // Добавим линию для значений ниже диапазона
        result.push_str("     ");
        for &value in &frequencies {
            result.push_str(if value < min_value { "┷" } else { "╵" });
            result.push_str("   ");
        }
        result.push('\n');

        result.push_str("     32 64 125 250 500  1k  2k  4k  8k 16k\n");

        result
    }

    pub fn presets(local: impl Into<String>) -> Vec<CreateSelectMenuOption> {
        let mut res = match local.into().as_str() {
            
            "ru" => vec![
                CreateSelectMenuOption::new("Стандартный", "default"),
            ],
            _ => vec![
                CreateSelectMenuOption::new("Default", "default")
            ]
        };
        res.append(&mut vec![
            CreateSelectMenuOption::new("Party", "party"),
            CreateSelectMenuOption::new("Club", "club"),
            CreateSelectMenuOption::new("Dance", "dance"),
            CreateSelectMenuOption::new("Deep", "deep"),
            CreateSelectMenuOption::new("Techno", "techno"),
            CreateSelectMenuOption::new("Electronic", "electronic"),
            CreateSelectMenuOption::new("Bass Reducer", "bass_reducer"),
            CreateSelectMenuOption::new("Bass Booster", "bass_booster"),
            CreateSelectMenuOption::new("Full Bass", "full_bass"),
            CreateSelectMenuOption::new("Treble Reducer", "treble_reducer"),
            CreateSelectMenuOption::new("Treble Booster", "treble_booster"),
            CreateSelectMenuOption::new("Full Treble", "full_treble"),
            CreateSelectMenuOption::new("Full Bass & Treble", "full_bass_and_treble"),
            CreateSelectMenuOption::new("Headphones", "headphones"),
            CreateSelectMenuOption::new("Laptop", "laptop"),
            CreateSelectMenuOption::new("Loudness", "loudness"),
            CreateSelectMenuOption::new("Lounge", "lounge"),
            CreateSelectMenuOption::new("Large Hall", "large_hall"),
            CreateSelectMenuOption::new("Acoustic", "acoustic"),
            CreateSelectMenuOption::new("Speech", "speech"),
            CreateSelectMenuOption::new("Vocal", "vocal"),
            CreateSelectMenuOption::new("Piano", "piano"),
            CreateSelectMenuOption::new("Classic", "classic"),
            CreateSelectMenuOption::new("Live", "live"),
            CreateSelectMenuOption::new("Pop", "pop"),
            CreateSelectMenuOption::new("R&B", "rnb"),
            CreateSelectMenuOption::new("Soft", "soft"),
            CreateSelectMenuOption::new("Soft Rock", "soft_rock"),
            CreateSelectMenuOption::new("Rock", "rock"),
            CreateSelectMenuOption::new("Alternative", "alternative"),
            CreateSelectMenuOption::new("Metal", "metal"),
            CreateSelectMenuOption::new("Indie", "indie"),
            CreateSelectMenuOption::new("Jazz", "jazz"),
            CreateSelectMenuOption::new("Ska", "ska"),
            CreateSelectMenuOption::new("Reggae", "reggae"),
            CreateSelectMenuOption::new("Hip-hop", "hiphop"),
            CreateSelectMenuOption::new("Latin", "latin")
        ]);
        res
    }

    pub fn get_preset(name: impl Into<String>) ->  &'static[f64] {
        match name.into().as_str() {
            "party" => &[4.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0, 4.0],
            "club" => &[0.0, 0.0, 5.0, 3.0, 3.0, 3.0, 2.0, 0.0, 0.0, 0.0],
            "dance" => &[6.0, 4.0, 1.0, 0.0, 0.0, -3.0, -4.0, -4.0, 0.0, 0.0],
            "deep" => &[5.0, 4.0, 2.0, 1.0, 3.0, 2.0, 2.0, -2.0, -3.0, -4.0],
            "techno" => &[5.0, 3.0, 0.0, -3.0, -3.0, 0.0, 5.0, 6.0, 6.0, 5.0],
            "electronic" => &[4.0, 4.0, 1.0, 0.0, -2.0, 2.0, 1.0, 1.0, 4.0, 5.0],
            "bass_reducer" => &[-4.0, -4.0, -3.0, -2.0, -1.0, 0.0, 0.0, 2.0, 3.0, 4.0],
            "bass_booster" => &[5.0, 4.0, 3.0, 2.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "full_bass" => &[5.0, 6.0, 6.0, 3.0, 1.0, -2.0, -5.0, -6.0, -7.0, -7.0],
            "treble_reducer" => &[0.0, 0.0, 0.0, 0.0, 0.0, -1.0, -2.0, -4.0, -4.0, -5.0],
            "treble_booster" => &[0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 4.0, 4.0, 5.0],
            "full_treble" => &[-6.0, -6.0, -6.0, -2.0, 1.0, 7.0, 10.0, 10.0, 10.0, 10.0],
            "full_bass_and_treble" => &[4.0, 3.0, 0.0, -4.0, -3.0, 1.0, 5.0, 7.0, 7.0, 7.0],
            "headphones" => &[3.0, 7.0, 3.0, -2.0, -1.0, 1.0, 3.0, 6.0, 8.0, 9.0],
            "laptop" => &[5.0, 4.0, 4.0, 2.0, 1.0, 0.0, -1.0, -3.0, -3.0, -4.0],
            "loudness" => &[6.0, 4.0, 0.0, 0.0, -2.0, 0.0, -1.0, -5.0, 5.0, 1.0],
            "large_hall" => &[6.0, 6.0, 3.0, 3.0, 0.0, -3.0, -3.0, -3.0, 0.0, 0.0],
            "acoustic" => &[5.0, 5.0, 3.0, 1.0, 2.0, 2.0, 3.0, 4.0, 3.0, 2.0],
            "speech" => &[-2.0, 0.0, 0.0, 1.0, 4.0, 5.0, 5.0, 4.0, 2.0, 0.0],
            "vocal" => &[-1.0, -3.0, -3.0, 1.0, 4.0, 4.0, 3.0, 2.0, 0.0, -1.0],
            "piano" => &[3.0, 2.0, 0.0, 3.0, 3.0, 2.0, 3.0, 5.0, 3.0, 3.0],
            "classic" => &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.0, -4.0, -4.0, -6.0],
            "live" => &[-3.0, 0.0, 2.0, 3.0, 3.0, 3.0, 2.0, 1.0, 1.0, 1.0],
            "pop" => &[1.0, 3.0, 4.0, 5.0, 3.0, 0.0, -1.0, -1.0, 1.0, 1.0],
            "rnb" => &[3.0, 7.0, 6.0, 1.0, -2.0, -1.0, 2.0, 3.0, 3.0, 4.0],
            "soft" => &[3.0, 1.0, 0.0, -1.0, 0.0, 2.0, 5.0, 6.0, 7.0, 7.0],
            "soft_rock" => &[2.0, 2.0, 1.0, 0.0, -2.0, -3.0, -2.0, 0.0, 1.0, 5.0],
            "rock" => &[5.0, 3.0, -3.0, -5.0, -2.0, 2.0, 5.0, 7.0, 7.0, 7.0],
            "alternative" => &[2.0, 2.0, 5.0, 0.0, -5.0, -5.0, 0.0, 0.0, 2.0, 5.0],
            "metal" => &[0.0, 0.0, 2.0, 2.0, -2.0, -5.0, -2.0, 0.0, 2.0, 0.0],
            "indie" => &[-2.0, -2.0, -2.0, -2.0, -2.0, 0.0, 2.0, 5.0, 2.0, 0.0],
            "jazz" => &[4.0, 3.0, 1.0, 2.0, -1.0, -1.0, 0.0, 1.0, 3.0, 4.0],
            "ska" => &[-1.0, -3.0, -2.0, 0.0, 2.0, 3.0, 5.0, 6.0, 7.0, 6.0],
            "reggae" => &[0.0, 0.0, 0.0, -3.0, 0.0, 4.0, 4.0, 0.0, 0.0, 0.0],
            "hiphop" => &[5.0, 4.0, 1.0, 3.0, -1.0, -1.0, 1.0, -1.0, 2.0, 3.0],
            "latin" => &[3.0, 2.0, 0.0, 0.0, -1.0, -1.0, -1.0, 0.0, 3.0, 5.0],
            _ => &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        }
    }
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