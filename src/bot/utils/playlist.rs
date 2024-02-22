use std::collections:: VecDeque;

use super::track::Track;

#[derive(Debug)]
pub struct Playlist {
    pub tracks: VecDeque<Track>,
    pub current: Option<Track>
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            tracks: VecDeque::new(),
            current: None
        }
    }

    pub fn add(&mut self, track: Track) {
        self.tracks.push_back(track);
    }

    pub fn get(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }
}