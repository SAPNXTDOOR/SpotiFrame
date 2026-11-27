use std::time::Instant;

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}

#[derive(Clone)]
pub struct TrackState {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub progress_ms: u64,
    pub synced_at: Instant,
    pub is_playing: bool,
    pub repeat_state: String, // "off", "context", or "track"
    pub cover_bytes: Option<Vec<u8>>,
    pub lyrics: Vec<LyricLine>,
    pub plain_lyrics: Option<String>,
}

impl TrackState {
    pub fn current_position_ms(&self) -> u64 {
        if self.is_playing {
            let elapsed = self.synced_at.elapsed().as_millis() as u64;
            (self.progress_ms + elapsed).min(self.duration_ms)
        } else {
            self.progress_ms
        }
    }

    pub fn active_line_index(&self) -> Option<usize> {
        let pos = self.current_position_ms();
        self.lyrics.iter().rposition(|line| line.time_ms <= pos)
    }
}

pub enum AppStatus {
    LoggedOut,
    Authenticating,
    Connected {
        track: Option<TrackState>,
        error: Option<String>,
    },
}