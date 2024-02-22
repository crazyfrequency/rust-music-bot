use std::sync::Arc;

use serenity::{all::GuildId, async_trait, client::Context};
use songbird::{events::{Event, EventContext, EventHandler as VoiceEventHandler}, input::Input, Songbird};

use crate::bot::utils::player::{PlayerData, PlayerState, Position, RepeatMode};

use super::utils::player::PlayerDataType;

pub struct TrackEndNotifier {
    pub guild_id: GuildId,
    pub ctx_clone: Context,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        println!("{:?}", ctx);
        if let EventContext::Track(track_list) = ctx {
            let player = self.ctx_clone.data.read().await.get::<PlayerData>().unwrap().clone();
            let player = player.read().await.clone();
            let player = player.get(&self.guild_id.get()).unwrap().clone();

            let manager = songbird::get(&self.ctx_clone).await.expect("Songbird Voice client placed in at initialisation.").clone();

            let mut player_playlist = player.playlist.write().await;
            let mut state = player.state.write().await;
            let settings = player.settings.read().await;
            let mut player_handler = player.player.write().await;

            if let Some(handler) = player_handler.clone() {
                if handler.uuid() != track_list[0].1.uuid(){
                    return None;
                }
            }
            
            match *state {
                PlayerState::Seeking => {
                    let mut position = player.position.write().await;
                    let track = player_playlist.current.clone();
                    println!("{}", track.is_some());
                    match track {
                        Some(track) => {
                            *state = PlayerState::Playing;
                            
                            let mut child = track.get_child(&self.ctx_clone, &self.guild_id.get(), position.last_position.as_secs_f64()).await.unwrap();
                            let stdin = child.stdin.take().unwrap();
                            let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                            if let Some(handler_lock) = manager.get(self.guild_id) {
                                let mut handler = handler_lock.lock().await;
                                let mut ffmpeg = player.ffmpeg.write().await;

                                let _ = ffmpeg.insert(stdin);

                                let handle = handler.play_only_input(data);
                                let _ = player_handler.insert(handle);
                            }
                        },
                        None => {
                            let track = player_playlist.tracks.pop_front();
                            if track.is_none() {
                                return None;
                            }
                            let track = track.unwrap();
                            player_playlist.current = Some(track.clone());
                            *state = PlayerState::Playing;
                            *position = Position::default();

                            let mut child = track.get_child(&self.ctx_clone, &self.guild_id.get(), 0.0).await.unwrap();
                            let stdin = child.stdin.take().unwrap();
                            let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                            if let Some(handler_lock) = manager.get(self.guild_id) {
                                let mut handler = handler_lock.lock().await;
                                let mut ffmpeg = player.ffmpeg.write().await;

                                let _ = ffmpeg.insert(stdin);

                                let handle = handler.play_only_input(data);
                                let _ = player_handler.insert(handle);
                            }
                        }
                    }
                },
                PlayerState::InSkip => {
                    let mut last_updated_position = player.position.write().await;
                        let track = player_playlist.tracks.pop_front();
                        if track.is_none() {
                            player_playlist.current = None;
                            *state = PlayerState::Ended;
                            return None;
                        }
                        let track = track.unwrap();
                        if settings.repeat==RepeatMode::Queue {
                            match player_playlist.current.clone() {
                                Some(track) => player_playlist.tracks.push_back(track),
                                None => {}
                            }
                        }
                        player_playlist.current = Some(track.clone());
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();

                        let mut child = track.get_child(&self.ctx_clone, &self.guild_id.get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                        if let Some(handler_lock) = manager.get(self.guild_id) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;

                            let _ = ffmpeg.insert(stdin);

                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                }
                _ => match settings.repeat {
                    RepeatMode::Track => {
                        let mut last_updated_position = player.position.write().await;
                        let track = player_playlist.current.clone();
                        if track.is_none() {
                            player_playlist.current = None;
                            *state = PlayerState::Ended;
                            return None;
                        }
                        let track = track.unwrap();
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();

                        let mut child = track.get_child(&self.ctx_clone, &self.guild_id.get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                        if let Some(handler_lock) = manager.get(self.guild_id) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;

                            let _ = ffmpeg.insert(stdin);

                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                    },
                    _ => {
                        let mut last_updated_position = player.position.write().await;
                        let track = player_playlist.tracks.pop_front();
                        if track.is_none() {
                            player_playlist.current = None;
                            *state = PlayerState::Ended;
                            return None;
                        }
                        let track = track.unwrap();
                        if settings.repeat==RepeatMode::Queue {
                            match player_playlist.current.clone() {
                                Some(track) => player_playlist.tracks.push_back(track),
                                None => {}
                            }
                        }
                        player_playlist.current = Some(track.clone());
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();

                        let mut child = track.get_child(&self.ctx_clone, &self.guild_id.get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                        if let Some(handler_lock) = manager.get(self.guild_id) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;

                            let _ = ffmpeg.insert(stdin);

                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                    }
                }
            };
        }
        else if let EventContext::DriverConnect(_) = ctx {
            let manager = songbird::get(&self.ctx_clone).await.expect("Songbird Voice client placed in at initialisation.").clone();
            if let Some(handler_lock) = manager.get(self.guild_id) {
                let mut handler = handler_lock.lock().await;
                let input = Input::from(vec![0,0,0]);
                handler.play_input(input);
                // let handle = handler.play_only_input(data);
            }
        }
        None
    }
}

pub struct TrackEndNotifierWeb {
    pub guild_id: GuildId,
    pub songbird: Arc<Arc<Songbird>>,
    pub player: Arc<PlayerDataType>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifierWeb {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        println!("{:?}", ctx);
        if let EventContext::Track(track_list) = ctx {
            let player = self.player.clone();
            let player = player.read().await.clone();
            let player = player.get(&self.guild_id.get()).unwrap().clone();

            let manager = self.songbird.clone();

            let mut player_playlist = player.playlist.write().await;
            let mut state = player.state.write().await;
            let settings = player.settings.read().await;
            let mut player_handler = player.player.write().await;

            if let Some(handler) = player_handler.clone() {
                if handler.uuid() != track_list[0].1.uuid(){
                    return None;
                }
            }
            
            match *state {
                PlayerState::Seeking => {
                    let mut position = player.position.write().await;
                    let track = player_playlist.current.clone();
                    println!("{}", track.is_some());
                    match track {
                        Some(track) => {
                            *state = PlayerState::Playing;
                            
                            let mut child = track.get_child_web(self.player.clone(), &self.guild_id.get(), position.last_position.as_secs_f64()).await.unwrap();
                            let stdin = child.stdin.take().unwrap();
                            let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                            if let Some(handler_lock) = manager.get(self.guild_id) {
                                let mut handler = handler_lock.lock().await;
                                let mut ffmpeg = player.ffmpeg.write().await;

                                let _ = ffmpeg.insert(stdin);

                                let handle = handler.play_only_input(data);
                                let _ = player_handler.insert(handle);
                            }
                        },
                        None => {
                            let track = player_playlist.tracks.pop_front();
                            if track.is_none() {
                                return None;
                            }
                            let track = track.unwrap();
                            player_playlist.current = Some(track.clone());
                            *state = PlayerState::Playing;
                            *position = Position::default();

                            let mut child = track.get_child_web(self.player.clone(), &self.guild_id.get(), 0.0).await.unwrap();
                            let stdin = child.stdin.take().unwrap();
                            let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                            if let Some(handler_lock) = manager.get(self.guild_id) {
                                let mut handler = handler_lock.lock().await;
                                let mut ffmpeg = player.ffmpeg.write().await;

                                let _ = ffmpeg.insert(stdin);

                                let handle = handler.play_only_input(data);
                                let _ = player_handler.insert(handle);
                            }
                        }
                    }
                },
                PlayerState::InSkip => {
                    let mut last_updated_position = player.position.write().await;
                        let track = player_playlist.tracks.pop_front();
                        if track.is_none() {
                            player_playlist.current = None;
                            *state = PlayerState::Ended;
                            return None;
                        }
                        let track = track.unwrap();
                        if settings.repeat==RepeatMode::Queue {
                            match player_playlist.current.clone() {
                                Some(track) => player_playlist.tracks.push_back(track),
                                None => {}
                            }
                        }
                        player_playlist.current = Some(track.clone());
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();

                        let mut child = track.get_child_web(self.player.clone(), &self.guild_id.get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                        if let Some(handler_lock) = manager.get(self.guild_id) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;

                            let _ = ffmpeg.insert(stdin);

                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                }
                _ => match settings.repeat {
                    RepeatMode::Track => {
                        let mut last_updated_position = player.position.write().await;
                        let track = player_playlist.current.clone();
                        if track.is_none() {
                            player_playlist.current = None;
                            *state = PlayerState::Ended;
                            return None;
                        }
                        let track = track.unwrap();
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();

                        let mut child = track.get_child_web(self.player.clone(), &self.guild_id.get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                        if let Some(handler_lock) = manager.get(self.guild_id) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;

                            let _ = ffmpeg.insert(stdin);

                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                    },
                    _ => {
                        let mut last_updated_position = player.position.write().await;
                        let track = player_playlist.tracks.pop_front();
                        if track.is_none() {
                            player_playlist.current = None;
                            *state = PlayerState::Ended;
                            return None;
                        }
                        let track = track.unwrap();
                        if settings.repeat==RepeatMode::Queue {
                            match player_playlist.current.clone() {
                                Some(track) => player_playlist.tracks.push_back(track),
                                None => {}
                            }
                        }
                        player_playlist.current = Some(track.clone());
                        *state = PlayerState::Playing;
                        *last_updated_position = Position::default();

                        let mut child = track.get_child_web(self.player.clone(), &self.guild_id.get(), 0.0).await.unwrap();
                        let stdin = child.stdin.take().unwrap();
                        let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));

                        if let Some(handler_lock) = manager.get(self.guild_id) {
                            let mut handler = handler_lock.lock().await;
                            let mut ffmpeg = player.ffmpeg.write().await;

                            let _ = ffmpeg.insert(stdin);

                            let handle = handler.play_only_input(data);
                            let _ = player_handler.insert(handle);
                        }
                    }
                }
            };
        }
        else if let EventContext::DriverConnect(_) = ctx {
            let manager = self.songbird.clone();
            if let Some(handler_lock) = manager.get(self.guild_id) {
                let mut handler = handler_lock.lock().await;
                let input = Input::from(vec![0,0,0]);
                handler.play_input(input);
                // let handle = handler.play_only_input(data);
            }
        }
        None
    }
}