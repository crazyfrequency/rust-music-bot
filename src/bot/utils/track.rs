use std::process::{Command, Stdio, Child};
use std::io::Error;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serenity::builder::CreateEmbed;
use serenity::client::Context;

use crate::bot::utils::player::PlayerData;

use super::player::PlayerDataType;

#[derive(Debug,Clone,Serialize)]
pub struct Track {
    pub id: u64,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub author: Author,
    pub url: String,
    pub views: Option<i64>,
    pub likes: Option<i64>,
    pub chapters: Vec<Chapter>,
    pub webpage_url: String,
    pub duration: Option<f64>,
    pub parse_time: DateTime<Utc>,
    pub parser_type: ParserType,
    pub edit_date: Option<DateTime<Utc>>,
}

impl Track {
    pub fn from_vk(track: VkTrack, id: u64) ->Self {
        Track {
            id,
            title: Some(track.title),
            description: None,
            thumbnail: None,
            author: Author {
                name: Some(track.author),
                url: None,
                thumbnail: None,
                verified: false,
            },
            url: track.url,
            views: None,
            likes: None,
            chapters: Vec::new(),
            webpage_url: track.webpage_url,
            duration: Some(track.duration),
            parse_time: Utc::now(),
            parser_type: ParserType::Vk,
            edit_date: None,
        }
    }

    pub fn from_web(track: WebTrack, id: u64) ->Self {
        Track {
            id,
            title: track.title,
            description: track.description,
            thumbnail: track.thumbnail,
            author: track.author,
            url: track.url,
            views: track.views,
            likes: track.likes,
            chapters: track.chapters,
            webpage_url: track.webpage_url,
            duration: track.duration,
            parse_time: track.parse_time,
            parser_type: track.parser_type,
            edit_date: track.edit_date
        }
    }
}

#[derive(Deserialize)]
pub struct WebTrack {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub author: Author,
    pub url: String,
    pub views: Option<i64>,
    pub likes: Option<i64>,
    pub chapters: Vec<Chapter>,
    pub webpage_url: String,
    pub duration: Option<f64>,
    pub parse_time: DateTime<Utc>,
    pub parser_type: ParserType,
    pub edit_date: Option<DateTime<Utc>>,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Author {
    pub name: Option<String>,
    pub url: Option<String>,
    pub thumbnail: Option<String>,
    pub verified: bool,
}

impl Track {
    pub async fn get_child(&self, ctx: &Context, guild_id: &u64, start: f64) -> Result<Child, Error> {
        let map = {
            let data_read = ctx.data.read().await;
            data_read.get::<PlayerData>().expect("Expected PlayerData in TypeMap.").clone()
        };
        let map = map.read().await;
        let player = map.get(guild_id).unwrap();
        let settings = player.settings.read().await;
        let command = &mut Command::new("ffmpeg");
        command.args([
            "-reconnect", "1", "-reconnect_streamed", "1",
            "-reconnect_delay_max", "5", "-err_detect", "ignore_err",
            "-vn", "-sn", "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:106.0) Gecko/20100101 Firefox/106.0"
            ]);
        if self.url.ends_with(".m3u8") {
            command.args(["-http_persistent", "false"]);
        };
        if start != 0.0 {
            command.args(["-ss", &format!("{}", start)]);
        };
        command.args(["-i", self.url.as_str()])
            .args(["-af", format!(
                "volume={},atempo{},bass=g={}:f=110:w=0.3,equalizer@h32=f=32:t=h:w=17{},equalizer@h64=f=64:t=h:w=30{},equalizer@h125=f=125:t=h:w=62{},equalizer@h250=f=250:t=h:w=125{},equalizer@h500=f=500:t=h:w=250{},equalizer@h1k=f=1k:t=h:w=500{},equalizer@h2k=f=2k:t=h:w=1000{},equalizer@h4k=f=4k:t=h:w=2000{},equalizer@h8k=f=8k:t=h:w=4000{},equalizer@h16k=f=16k:t=h:w=8000{}",
                settings.volume*0.2,
                if settings.speed != 1.0 {format!("={}", settings.speed)} else {"".to_string()},
                if settings.bass_enabled {format!("{}", settings.bass_gain)} else {"0".to_string()},
                if settings.equalizer.f_32 != 0.0 {format!(":g={}", settings.equalizer.f_32)} else {"".to_string()},
                if settings.equalizer.f_64 != 0.0 {format!(":g={}", settings.equalizer.f_64)} else {"".to_string()},
                if settings.equalizer.f_125 != 0.0 {format!(":g={}", settings.equalizer.f_125)} else {"".to_string()},
                if settings.equalizer.f_250 != 0.0 {format!(":g={}", settings.equalizer.f_250)} else {"".to_string()},
                if settings.equalizer.f_500 != 0.0 {format!(":g={}", settings.equalizer.f_500)} else {"".to_string()},
                if settings.equalizer.f_1k != 0.0 {format!(":g={}", settings.equalizer.f_1k)} else {"".to_string()},
                if settings.equalizer.f_2k != 0.0 {format!(":g={}", settings.equalizer.f_2k)} else {"".to_string()},
                if settings.equalizer.f_4k != 0.0 {format!(":g={}", settings.equalizer.f_4k)} else {"".to_string()},
                if settings.equalizer.f_8k != 0.0 {format!(":g={}", settings.equalizer.f_8k)} else {"".to_string()},
                if settings.equalizer.f_16k != 0.0 {format!(":g={}", settings.equalizer.f_16k)} else {"".to_string()},
            ).as_str()])
            .args(["-f", "wav", "-loglevel","info", "pipe:1"]);
        command.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
    }

    pub async fn get_child_web(&self, player_data: Arc<PlayerDataType>, guild_id: &u64, start: f64) -> Result<Child, Error> {
        let map = player_data.read().await;
        let player = map.get(guild_id).unwrap();
        let settings = player.settings.read().await;
        let command = &mut Command::new("ffmpeg");
        command.args([
            "-reconnect", "1", "-reconnect_streamed", "1",
            "-reconnect_delay_max", "5", "-err_detect", "ignore_err",
            "-vn", "-sn", "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:106.0) Gecko/20100101 Firefox/106.0"
            ]);
        if self.url.ends_with(".m3u8") {
            command.args(["-http_persistent", "false"]);
        };
        if start != 0.0 {
            command.args(["-ss", &format!("{}", start)]);
        };
        command.args(["-i", self.url.as_str()])
            .args(["-af", format!(
                "volume={},atempo{},bass=g={}:f=110:w=0.3,equalizer@h32=f=32:t=h:w=17{},equalizer@h64=f=64:t=h:w=30{},equalizer@h125=f=125:t=h:w=62{},equalizer@h250=f=250:t=h:w=125{},equalizer@h500=f=500:t=h:w=250{},equalizer@h1k=f=1k:t=h:w=500{},equalizer@h2k=f=2k:t=h:w=1000{},equalizer@h4k=f=4k:t=h:w=2000{},equalizer@h8k=f=8k:t=h:w=4000{},equalizer@h16k=f=16k:t=h:w=8000{}",
                settings.volume*0.2,
                if settings.speed != 1.0 {format!("={}", settings.speed)} else {"".to_string()},
                if settings.bass_enabled {format!("{}", settings.bass_gain)} else {"0".to_string()},
                if settings.equalizer.f_32 != 0.0 {format!(":g={}", settings.equalizer.f_32)} else {"".to_string()},
                if settings.equalizer.f_64 != 0.0 {format!(":g={}", settings.equalizer.f_64)} else {"".to_string()},
                if settings.equalizer.f_125 != 0.0 {format!(":g={}", settings.equalizer.f_125)} else {"".to_string()},
                if settings.equalizer.f_250 != 0.0 {format!(":g={}", settings.equalizer.f_250)} else {"".to_string()},
                if settings.equalizer.f_500 != 0.0 {format!(":g={}", settings.equalizer.f_500)} else {"".to_string()},
                if settings.equalizer.f_1k != 0.0 {format!(":g={}", settings.equalizer.f_1k)} else {"".to_string()},
                if settings.equalizer.f_2k != 0.0 {format!(":g={}", settings.equalizer.f_2k)} else {"".to_string()},
                if settings.equalizer.f_4k != 0.0 {format!(":g={}", settings.equalizer.f_4k)} else {"".to_string()},
                if settings.equalizer.f_8k != 0.0 {format!(":g={}", settings.equalizer.f_8k)} else {"".to_string()},
                if settings.equalizer.f_16k != 0.0 {format!(":g={}", settings.equalizer.f_16k)} else {"".to_string()},
            ).as_str()])
            .args(["-f", "wav", "-loglevel","info", "pipe:1"]);
        command.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
    }

    pub fn get_embed(&self, local: &str) -> CreateEmbed {
        let title = match &self.title {
            Some(title) => title.clone(),
            None => match local {
                "ru" => "Название не известно",
                _ => "Unknown title"
            }.to_string()
        };
        let author = match &self.author.name {
            Some(author) => author.clone(),
            None => match local {
                "ru" => "Автор не известен",
                _ => "Unknown author"
            }.to_string()
        };
        let author = match &self.author.url {
            Some(url) => format!("[{}]({})", author, url),
            None => author
        };
        let title = if self.webpage_url.is_empty() {
            title
        } else {
            format!("[{}]({})", title, self.webpage_url)
        };
        let mut embed = CreateEmbed::default()
            .field(match local {
                "ru" => "Название",
                _ => "Title",
            }, title, false)
            .field(match local {
                "ru" => "Автор",
                _ => "Author",
            }, match &self.author.verified {
                true => format!("{} ✔", author),
                false => author
            }, true)
            .image(match &self.thumbnail {
                Some(url) => url.clone(),
                None => "https://cdn.discordapp.com/attachments/911499726644477992/975252886416146452/undefinded.png".to_string()
            });
        if let Some(date) = self.edit_date {
            embed = embed.timestamp(date);
        }
        if let Some(likes) = self.likes {
            embed = embed.field(match local {
                "ru" => "Лайков",
                _ => "Likes",
            }, likes.to_string(), true)
        }
        if let Some(views) = self.views {
            embed = embed.field(match local {
                "ru" => "Просмотров",
                _ => "Views",
            }, views.to_string(), true)
        }
        embed
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Chapter {
    pub title: String,
    pub start_time: f64,
    pub start_time_str: String,
    pub end_time: f64,
}

#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
pub enum ParserType {
    YtDl,
    Ffprobe,
    Vk
}

#[derive(Debug,Clone)]
pub struct YtDlTracksPlaylist {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub author: Author,
    pub views: Option<i64>,
    pub webpage_url: String,
    pub edit_date: Option<DateTime<Utc>>,
    pub tracks: Vec<String>
}

#[derive(Debug,Clone)]
pub struct VkTracksPlaylist {
    pub tracks: Vec<VkTrack>
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct VkTrack {
    pub title: String,
    pub author: String,
    pub duration: f64,
    pub webpage_url: String,
    pub url: String,
}

pub enum PlaylistType {
    Vk(VkTracksPlaylist),
    YtDl(YtDlTracksPlaylist),
    None
}