use chrono::{DateTime, Utc, TimeZone};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use youtube_dl::SearchType;

use crate::bot::utils::track::VkTrack;

use super::track::{Track, ParserType, Author, Chapter, YtDlTracksPlaylist, VkTracksPlaylist};

pub async fn parse_url(url: impl Into<&str>, track_id: u64) -> ParsedDataType {
    let url = url.into();
    let vk_url = match Regex::new(r"^(http:\/\/|https:\/\/)?(www.)?(vk\.com|vkontakte\.ru)\/") {
        Ok(re) => re.is_match(url),
        _ => false
    };

    let raw_result = if vk_url {
        youtube_dl::YoutubeDl::new(url).youtube_dl_path("python").extra_arg("./vk_parser.py").run_raw_async().await
    } else {
        youtube_dl::YoutubeDl::new(url).flat_playlist(true).socket_timeout("15").run_raw_async().await
    };

    let playlist_type = match &raw_result {
        Ok(src) => match src.get("_type") {
            Some(src) => src.clone() == json!("playlist"),
            None => return ParsedDataType::None
        },
        Err(_) => return ParsedDataType::None
    };
    let raw_result = raw_result.unwrap();

    if playlist_type {
        if vk_url {
            todo!()
        } else {
            let playlist = parse_playlist_yt(raw_result).await;
            match playlist {
                Some(playlist) => match playlist.tracks.first() {
                    Some(url) => match youtube_dl::YoutubeDl::new(url).flat_playlist(true).socket_timeout("15").run_raw_async().await {
                        Ok(src) => match parse_track_yt(src, track_id).await {
                            Some(track) => 
                                return ParsedDataType::YtDlPlaylist((playlist, track)),
                            None => return ParsedDataType::None
                        },
                        Err(_) => return ParsedDataType::None
                    },
                    None => return ParsedDataType::None
                },
                None => return ParsedDataType::None
            };
        };
    } else {
        if vk_url {
            match serde_json::from_value::<VkTrack>(raw_result) {
                Ok(track) => return ParsedDataType::Track(Track::from_vk(track, track_id)),
                Err(_) => return ParsedDataType::None
            }
        } else {
            match parse_track_yt(raw_result, track_id).await {
                Some(track) => return ParsedDataType::Track(track),
                None => return ParsedDataType::None
            }
        }
    }
}

pub async fn search_track_yt(query: impl Into<&str>, track_id: u64, search_type: SearchType) -> Option<Track> {
    let options = match search_type {
        SearchType::SoundCloud => youtube_dl::SearchOptions::soundcloud(query.into()).with_count(1),
        _ => youtube_dl::SearchOptions::youtube(query.into()).with_count(1)
    };
    match youtube_dl::YoutubeDl::search_for(&options).socket_timeout("15").run_raw_async().await {
        Ok(data) => match data.get("entries") {
            Some(Value::Array(data)) => match data.first() {
                Some(data) => return parse_track_yt(data.clone(), track_id).await,
                None => return None
            },
            _ => return None
        },
        Err(_) => return None
    };
}

pub async fn search_track_vk(query: impl Into<&str>, track_id: u64) -> Option<Track> {
    match youtube_dl::YoutubeDl::new(format!("search:{}", query.into())).youtube_dl_path("python").extra_arg("./vk_parser.py").run_raw_async().await {
        Ok(data) => match serde_json::from_value::<VkTrack>(data) {
            Ok(track) => Some(Track::from_vk(track, track_id)),
            Err(_) => None
        },
        Err(_) => None
    }
}

#[derive(Debug,Clone)]
pub enum ParsedDataType {
    YtDlPlaylist((YtDlTracksPlaylist, Track)),
    VkPlaylist((VkTracksPlaylist, Track)),
    Track(Track),
    None
}

pub async fn parse_track_yt(data: Value, id: u64) -> Option<Track> {
    let data = match data.as_object() {
        Some(data) => data,
        None => return None
    };
    
    let webpage_url = match data.get("webpage_url") {
        Some(Value::String(url)) => Some(url.clone()),
        _ => match data.get("original_url") {
            Some(Value::String(url)) => Some(url.clone()),
            _ => None
        }
    };
    if webpage_url.is_none() {
        return None;
    };
    let webpage_url = webpage_url.unwrap();
    let url = match data.get("formats") {
        Some(Value::Array(formats)) => if webpage_url.starts_with("https://www.twitch.tv") {
            find_best_audio_twitch(formats.clone())
        } else {
            find_best_audio(formats.clone())
        },
        _ => match data.get("url") {
            Some(Value::String(url)) => 
                Some(url.clone()),
            _ => None
        }
    };
    if url.is_none() {
        return None;
    };
    let url = url.unwrap();
    let thumbnail = match data.get("thumbnail") {
        Some(Value::String(url)) => Some(url.clone()),
        _ => match data.get("thumbnails") {
            Some(Value::Array(thumbnails)) => find_best_thumbnail(thumbnails.clone()),
            _ => None
        }
    };
    let mut edit_date = match data.get("modified_timestamp") {
        Some(Value::Number(edit_date)) =>
            match Utc.timestamp_micros((edit_date.as_f64().unwrap_or(0.0) as i64) * 1000 * 1000) {
                chrono::LocalResult::Single(edit_date) => Some(edit_date),
                _ => None
            },
        _ => None
    };if edit_date.is_none() {
        edit_date = match data.get("modified_date") {
            Some(Value::String(date)) =>
                match DateTime::parse_from_str(date, "%Y%m%d") {
                    Ok(edit_date) => Some(edit_date.into()),
                    _ => None
                },
            _ => None
        };
    };if edit_date.is_none() {
        edit_date = match data.get("release_timestamp") {
            Some(Value::Number(edit_date)) =>
                match Utc.timestamp_micros((edit_date.as_f64().unwrap_or(0.0) as i64) * 1000 * 1000) {
                    chrono::LocalResult::Single(edit_date) => Some(edit_date),
                    _ => None
                }
            _ => None
        }
    };if edit_date.is_none() {
        edit_date = match data.get("release_date") {
            Some(Value::String(date)) =>
                match DateTime::parse_from_str(date, "%Y%m%d") {
                    Ok(edit_date) => Some(edit_date.into()),
                    _ => None
                }
            _ => None
        }
    };if edit_date.is_none() {
        edit_date = match data.get("timestamp") {
            Some(Value::Number(edit_date)) =>
                match Utc.timestamp_micros((edit_date.as_f64().unwrap_or(0.0) as i64) * 1000 * 1000) {
                    chrono::LocalResult::Single(edit_date) => Some(edit_date),
                    _ => None
                }
            _ => None
        }
    };if edit_date.is_none() {
        edit_date = match data.get("upload_date") {
            Some(Value::String(date)) =>
                match DateTime::parse_from_str(date, "%Y%m%d") {
                    Ok(edit_date) => Some(edit_date.into()),
                    _ => None
                }
            _ => None
        }
    };let duration = match data.get("duration") {
        Some(Value::Number(duration)) => duration.as_f64(),
        Some(Value::String(duration)) => Some(get_time(duration)),
        _ => match data.get("duration_string") {
            Some(Value::String(duration)) => Some(get_time(duration)),
            _ => None
        }
    };
    Some(
        Track {
            id,
            title: match data.get("title") {
                Some(Value::String(title)) => Some(title.clone()),
                _ => None
            },
            description: match data.get("description") {
                Some(Value::String(description)) => Some(description.clone()),
                _ => None
            },
            thumbnail,
            author: Author {
                name: match data.get("channel") {
                    Some(Value::String(channel)) => Some(channel.clone()),
                    _ => match data.get("uploader") {
                        Some(Value::String(uploader)) => Some(uploader.clone()),
                        _ => match data.get("artist") {
                            Some(Value::String(artist)) => Some(artist.clone()),
                            _ => None
                        }
                    }
                },
                url: match data.get("channel_url") {
                    Some(Value::String(channel_url)) => Some(channel_url.clone()),
                    _ => match data.get("uploader_url") {
                        Some(Value::String(uploader_url)) => Some(uploader_url.clone()),
                        _ => None
                    }
                },
                thumbnail: None, // get_avatar(author_url).await
                verified: match data.get("channel_verified") {
                    Some(Value::Bool(channel_verified)) => *channel_verified,
                    _ => false
                }
            },
            url,
            views: match data.get("view_count") {
                Some(Value::Number(view_count)) => view_count.as_i64(),
                Some(Value::String(view_count)) => match view_count.parse::<i64>() {
                    Ok(view_count) => Some(view_count as i64),
                    _ => None
                },
                _ => None
            },
            likes: match data.get("like_count") {
                Some(Value::Number(like_count)) => like_count.as_i64(),
                Some(Value::String(like_count)) => match like_count.parse::<i64>() {
                    Ok(like_count) => Some(like_count as i64),
                    _ => None
                },
                _ => None
            },
            chapters: match data.get("chapters") {
                Some(chapters) => match chapters {
                    Value::Array(_) => get_chapters(chapters.clone(), duration.unwrap_or(0.0)),
                    _ => Vec::new()
                },
                _ => Vec::new()
            },
            webpage_url,
            duration,
            parse_time: Utc::now(),
            parser_type: ParserType::YtDl,
            edit_date,
        }
    )
}

pub async fn parse_playlist_yt(data: Value) -> Option<YtDlTracksPlaylist> {
    let data = match data.as_object() {
        Some(data) => data,
        None => return None
    };
    
    let webpage_url = match data.get("webpage_url") {
        Some(Value::String(url)) => Some(url.clone()),
        _ => match data.get("original_url") {
            Some(Value::String(url)) => Some(url.clone()),
            _ => None
        }
    };
    if webpage_url.is_none() {
        return None;
    }
    let webpage_url = webpage_url.unwrap();
    let thumbnail = match data.get("thumbnail") {
        Some(Value::String(url)) => Some(url.clone()),
        _ => match data.get("thumbnails") {
            Some(Value::Array(thumbnails)) => find_best_thumbnail(thumbnails.clone()),
            _ => None
        }
    };
    let mut edit_date = match data.get("modified_timestamp") {
        Some(Value::Number(edit_date)) =>
            match Utc.timestamp_micros((edit_date.as_f64().unwrap_or(0.0) as i64) * 1000 * 1000) {
                chrono::LocalResult::Single(edit_date) => Some(edit_date),
                _ => None
            },
        _ => None
    };if edit_date.is_none() {
        edit_date = match data.get("modified_date") {
            Some(Value::String(date)) =>
                match DateTime::parse_from_str(date, "%Y%m%d") {
                    Ok(edit_date) => Some(edit_date.into()),
                    _ => None
                },
            _ => None
        };
    };if edit_date.is_none() {
        edit_date = match data.get("release_timestamp") {
            Some(Value::Number(edit_date)) =>
                match Utc.timestamp_micros((edit_date.as_f64().unwrap_or(0.0) as i64) * 1000 * 1000) {
                    chrono::LocalResult::Single(edit_date) => Some(edit_date),
                    _ => None
                }
            _ => None
        }
    };if edit_date.is_none() {
        edit_date = match data.get("release_date") {
            Some(Value::String(date)) =>
                match DateTime::parse_from_str(date, "%Y%m%d") {
                    Ok(edit_date) => Some(edit_date.into()),
                    _ => None
                }
            _ => None
        }
    };if edit_date.is_none() {
        edit_date = match data.get("timestamp") {
            Some(Value::Number(edit_date)) =>
                match Utc.timestamp_micros((edit_date.as_f64().unwrap_or(0.0) as i64) * 1000 * 1000) {
                    chrono::LocalResult::Single(edit_date) => Some(edit_date),
                    _ => None
                }
            _ => None
        }
    };if edit_date.is_none() {
        edit_date = match data.get("upload_date") {
            Some(Value::String(date)) =>
                match DateTime::parse_from_str(date, "%Y%m%d") {
                    Ok(edit_date) => Some(edit_date.into()),
                    _ => None
                }
            _ => None
        }
    };

    let tracks = match data.get("entries") {
        Some(Value::Array(tracks)) => {
            let mut urls: Vec<String> = Vec::with_capacity(tracks.len());
            for track in tracks {
                match track {
                    Value::Object(track) => {
                        match track.get("url") {
                            Some(Value::String(url)) => urls.push(url.clone()),
                            _ => {}
                        };
                    },
                    _ => {}
                };
            };
            urls
        },
        _ => Vec::new()
    };

    Some(YtDlTracksPlaylist {
        title: match data.get("title") {
            Some(Value::String(title)) => Some(title.clone()),
            _ => None
        },
        description: match data.get("description") {
            Some(Value::String(description)) => Some(description.clone()),
            _ => None
        },
        thumbnail,
        author: Author {
            name: match data.get("channel") {
                Some(Value::String(channel)) => Some(channel.clone()),
                _ => match data.get("uploader") {
                    Some(Value::String(uploader)) => Some(uploader.clone()),
                    _ => match data.get("artist") {
                        Some(Value::String(artist)) => Some(artist.clone()),
                        _ => None
                    }
                }
            },
            url: match data.get("channel_url") {
                Some(Value::String(channel_url)) => Some(channel_url.clone()),
                _ => match data.get("uploader_url") {
                    Some(Value::String(uploader_url)) => Some(uploader_url.clone()),
                    _ => None
                }
            },
            thumbnail: None, // get_avatar(author_url).await
            verified: match data.get("channel_verified") {
                Some(Value::Bool(channel_verified)) => *channel_verified,
                _ => false
            }
        },
        views: match data.get("view_count") {
            Some(Value::Number(view_count)) => view_count.as_i64(),
            Some(Value::String(view_count)) => match view_count.parse::<i64>() {
                Ok(view_count) => Some(view_count as i64),
                _ => None
            },
            _ => None
        },
        webpage_url,
        edit_date,
        tracks
    })
}

pub fn get_time(time: impl Into<String>) -> f64 {
    let mut time = time.into();
    let mut total_time: u64 = 0;

    if time.contains(&['h', 'm', 's'][..]) {
        if let Some(p) = time.find('h') {
            total_time = total_time * 60 + time[..p].parse::<u64>().unwrap_or(0);
            time.drain(..p + 1);
        }

        if let Some(p) = time.find('m') {
            total_time = total_time * 60 + time[..p].parse::<u64>().unwrap_or(0);
            time.drain(..p + 1);
        }

        if let Some(p) = time.find('s') {
            total_time = total_time * 60 + time[..p].parse::<u64>().unwrap_or(0);
            time.drain(..p + 1);
        }

        return total_time as f64;
    } else {
        if time.find(':').is_none() {
            return time.parse::<u64>().unwrap_or(0) as f64;
        } else {
            while let Some(p) = time.find(':') {
                total_time = total_time * 60 + time[..p].parse::<u64>().unwrap_or(0);
                time.drain(..p + 1);
            }

            total_time = total_time * 60 + time.parse::<u64>().unwrap_or(0);
            return total_time as f64;
        }
    }
}

pub fn get_time_str(time: f64) -> String {
    let time_seconds = time as i64;

    if time_seconds <= 0 {
        return time_seconds.to_string();
    }

    let hours = time_seconds / 3600;
    let minutes = (time_seconds % 3600) / 60;
    let seconds = time_seconds % 60;

    format!(
        "{}{:02}:{:02}",
        if hours > 0 { format!("{}:", hours) } else { String::new() },
        minutes,
        seconds
    )
}

fn get_chapters(data: Value, duration: f64) -> Vec<Chapter> {
    let data: Vec<ChapterValue> = match serde_json::from_value(data) {
        Ok(data) => data,
        Err(_) => return Vec::new()
    };
    let mut chapters: Vec<Chapter> = Vec::with_capacity(data.len());
    for (chapter, next_chapter) in data.iter().zip(data.iter().skip(1)) {
        match chapter.end_time {
            Some(time) => chapters.push(Chapter {
                title: chapter.title.clone(),
                start_time: chapter.start_time,
                start_time_str: get_time_str(chapter.start_time),
                end_time: time
            }),
            None => chapters.push(Chapter {
                title: chapter.title.clone(),
                start_time: chapter.start_time,
                start_time_str: get_time_str(chapter.start_time),
                end_time: next_chapter.start_time
            })
        }
    }
    match data.last() {
        Some(chapter) => match chapter.end_time {
            Some(time) => chapters.push(Chapter {
                title: chapter.title.clone(),
                start_time: chapter.start_time,
                start_time_str: get_time_str(chapter.start_time),
                end_time: time
            }),
            None => chapters.push(Chapter {
                title: chapter.title.clone(),
                start_time: chapter.start_time,
                start_time_str: get_time_str(chapter.start_time),
                end_time: duration
            })
        },
        None => {}
    }
    chapters
}

fn find_best_audio(formats: Vec<Value>) -> Option<String> {
    let mut best_abr: f64 = 0.0;
    let mut best_url: Option<String> = None;
    for format in &formats {
        match format.get("abr") {
            Some(Value::Number(value)) => {
                let value = value.as_f64().unwrap_or(0.0);
                if value > best_abr {
                    if let Some(Value::String(url)) = format.get("url") {
                        best_abr = value;
                        best_url = Some(url.clone());
                    }
                }
            },
            Some(Value::String(value)) => match value.parse::<f64>() {
                Ok(value) => if value > best_abr {
                    if let Some(Value::String(url)) = format.get("url") {
                        best_abr = value;
                        best_url = Some(url.clone());
                    }
                },
                _ => {}
            }
            _ => {}
        }
    }
    match best_url {
        Some(url) => Some(url),
        None => {
            for format in formats {
                if let Some(Value::String(url)) = format.get("url") {
                    return Some(url.clone());
                }
            }
            None
        }
    }
}

fn find_best_audio_twitch(formats: Vec<Value>) -> Option<String> {
    for format in &formats {
        match format.get("tbr") {
            Some(Value::Number(value)) => {
                let value = value.as_f64().unwrap_or(0.0);
                if value > 700.0 {
                    if let Some(Value::String(url)) = format.get("url") {
                        return Some(url.clone());
                    }
                }
            },
            Some(Value::String(value)) => match value.parse::<f64>() {
                Ok(value) => if value > 700.0 {
                    if let Some(Value::String(url)) = format.get("url") {
                        return Some(url.clone());
                    }
                },
                _ => {}
            }
            _ => {}
        }
    }
    None
}

fn find_best_thumbnail(thumbnails: Vec<Value>) -> Option<String> {
    let mut best_height: f64 = 0.0;
    let mut best_url: Option<String> = None;
    for thumbnail in thumbnails {
        if let Some(Value::String(url)) = thumbnail.get("url") {
            match thumbnail.get("height") {
                Some(Value::Number(height)) => {
                    let height = height.as_f64().unwrap_or(0.0);
                    if best_height < height {
                        best_height = height;
                        best_url = Some(url.clone());
                    }
                },
                Some(Value::String(height)) => {
                    let height = height.parse::<f64>().unwrap_or(0.0);
                    if best_height < height {
                        best_height = height;
                        best_url = Some(url.clone());
                    }
                },
                _ => match best_url {
                    Some(_) => (),
                    None => best_url = Some(url.clone())
                }
            }
        };
        
    }
    best_url
}

// async fn get_avatar(author_url: Option<impl Into<&str>>) -> Option<String> {
//     use std::process::Stdio;
//     use tokio::process::Command;
//     use tokio::time::timeout;
//     if author_url.is_none() {
//         return None;
//     }
//     let author_url = author_url.unwrap().into();

//     let child = Command::new("yt-dlp")
//         .args(["--skip-download", author_url, "--list-thumb", "--playlist-items", "0"])
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .stdin(Stdio::null())
//         .spawn();
//     let mut child = match child {
//         Ok(c) => c,
//         Err(_) => return None
//     };
//     let mut stdout = Vec::new();
//     let mut child_stdout = match child.stdout.take() {
//         Some(s) => s,
//         None => return None
//     };
//     let _ = tokio::io::copy(&mut child_stdout, &mut stdout).await;
//     let data = match timeout(Duration::from_secs(5), child.wait()).await {
//         Ok(n) => match n {
//             Ok(n) => n,
//             Err(_) => return None
//         },
//         Err(_) => {
//             let _ = child.kill().await;
//             return None;
//         }
//     };
//     None
// }

#[derive(Serialize, Deserialize)]
struct ChapterValue {
    pub title: String,
    pub start_time: f64,
    pub end_time: Option<f64>,
}