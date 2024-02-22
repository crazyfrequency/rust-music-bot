use diesel::prelude::*;

#[derive(Insertable, Selectable, Queryable, Identifiable, Debug, Clone, Copy)]
#[diesel(table_name = crate::schema::guild_settings)]
pub struct GuildSettingsDB {
    pub id: i64,
    pub speed: f64,
    pub volume: f64,
    pub loop_type: i16,
    pub bass_enabled: bool,
    pub bass_gain: f64,
    pub equalizer_32: f64,
    pub equalizer_64: f64,
    pub equalizer_125: f64,
    pub equalizer_250: f64,
    pub equalizer_500: f64,
    pub equalizer_1k: f64,
    pub equalizer_2k: f64,
    pub equalizer_4k: f64,
    pub equalizer_8k: f64,
    pub equalizer_16k: f64,
}

impl GuildSettingsDB {
    pub fn new(guild_id: u64) -> Self {
        Self {
            id: guild_id as i64,
            speed: 1.0,
            volume: 1.0,
            loop_type: 0,
            bass_enabled: false,
            bass_gain: 0.0,
            equalizer_32: 0.0,
            equalizer_64: 0.0,
            equalizer_125: 0.0,
            equalizer_250: 0.0,
            equalizer_500: 0.0,
            equalizer_1k: 0.0,
            equalizer_2k: 0.0,
            equalizer_4k: 0.0,
            equalizer_8k: 0.0,
            equalizer_16k: 0.0,
        }
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::guild_settings)]
pub struct UpdateBass {
    pub bass_enabled: bool,
    pub bass_gain: f64,
}