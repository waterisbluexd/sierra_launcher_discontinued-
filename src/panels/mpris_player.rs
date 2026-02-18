use mpris::{Player, PlayerFinder, PlaybackStatus};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MusicPlayerState {
    pub app_name: String,
    pub song_name: String,
    pub artist_name: String,
    pub current_time: f32,
    pub total_time: f32,
    pub is_playing: bool,
    pub player_available: bool,
    pub thumbnail_path: Option<String>,
}

pub struct MusicPlayerInternal {
    player: Option<Player>,
    last_check: std::time::Instant,
    cached_position: Option<i64>,
    last_status: Option<PlaybackStatus>,
}

impl Default for MusicPlayerState {
    fn default() -> Self {
        Self {
            app_name: "No Player".to_string(),
            song_name: "No Music Playing".to_string(),
            artist_name: "Start playing music in Spotify, YouTube, or any MPRIS-compatible player".to_string(),
            current_time: 0.0,
            total_time: 0.0,
            is_playing: false,
            player_available: false,
            thumbnail_path: None,
        }
    }
}

impl Default for MusicPlayerInternal {
    fn default() -> Self {
        Self {
            player: None,
            last_check: std::time::Instant::now(),
            cached_position: None,
            last_status: None,
        }
    }
}

pub struct MusicPlayer {
    pub state: MusicPlayerState,
    internal: MusicPlayerInternal,
}

impl MusicPlayer {
    pub fn new() -> Self {
        let mut player = Self {
            state: MusicPlayerState::default(),
            internal: MusicPlayerInternal::default(),
        };
        player.refresh_player();
        player
    }

    fn find_active_player() -> Option<Player> {
        let finder = PlayerFinder::new().ok()?;

        for player in finder.find_all().ok()? {
            if let Ok(status) = player.get_playback_status() {
                if status == PlaybackStatus::Playing || status == PlaybackStatus::Paused {
                    return Some(player);
                }
            }
        }

        None
    }

    pub fn refresh_player(&mut self) {
        if self.internal.last_check.elapsed() > Duration::from_secs(2) {
            self.internal.player = Self::find_active_player();
            self.internal.last_check = std::time::Instant::now();
            self.update_player_info();
        }
    }

    fn update_player_info(&mut self) {
        if let Some(player) = &self.internal.player {
            if let Ok(metadata) = player.get_metadata() {
                if let Ok(status) = player.get_playback_status() {
                    self.state.app_name = player.identity().to_string();

                    self.state.song_name = Self::truncate_text(
                        metadata.title()
                            .unwrap_or("Unknown"),
                        27
                    );

                    let artist = metadata
                        .artists()
                        .and_then(|artists| artists.first().map(|s| s.to_string()))
                        .unwrap_or_else(|| "Unknown Artist".to_string());
                    self.state.artist_name = Self::truncate_text(&artist, 25);

                    self.state.total_time = metadata.length()
                        .map(|l| (l.as_micros() as f64 / 1_000_000.0) as f32)
                        .unwrap_or(0.0);

                    // Get thumbnail/artwork URL
                    self.state.thumbnail_path = metadata.art_url()
                        .map(|url| {
                            let url_str = url.to_string();
                            // Remove "file://" prefix if present
                            if url_str.starts_with("file://") {
                                url_str.strip_prefix("file://").unwrap_or(&url_str).to_string()
                            } else {
                                url_str
                            }
                        });

                    let position = if status == PlaybackStatus::Paused {
                        if self.internal.last_status != Some(PlaybackStatus::Paused) {
                            self.internal.cached_position = player.get_position()
                                .ok()
                                .map(|p| p.as_micros() as i64);
                        }
                        self.internal.cached_position.unwrap_or(0)
                    } else {
                        self.internal.cached_position = None;
                        player.get_position()
                            .ok()
                            .map(|p| p.as_micros() as i64)
                            .unwrap_or(0)
                    };

                    self.state.current_time = (position as f64 / 1_000_000.0) as f32;

                    self.state.is_playing = status == PlaybackStatus::Playing;
                    self.internal.last_status = Some(status);
                    self.state.player_available = true;
                    return;
                }
            }
        }

        self.state.player_available = false;
        self.state.app_name = "No Player".to_string();
        self.state.song_name = "No Music Playing".to_string();
        self.state.artist_name = "Start playing music in Spotify, YouTube, or any MPRIS-compatible player".to_string();
        self.state.current_time = 0.0;
        self.state.total_time = 0.0;
        self.state.is_playing = false;
        self.state.thumbnail_path = None;
    }

    pub fn format_time(seconds: f32) -> String {
        let total_seconds = seconds as u32;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{}:{:02}", minutes, secs)
        }
    }

    fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            format!("{}...", &text[..max_len - 3])
        } else {
            text.to_string()
        }
    }

    pub fn play_pause(&mut self) -> bool {
        if let Some(player) = &self.internal.player {
            if player.play_pause().is_ok() {
                self.internal.cached_position = None;
                std::thread::sleep(Duration::from_millis(100));
                self.update_player_info();
                return true;
            }
        }
        false
    }

    pub fn next_track(&mut self) -> bool {
        if let Some(player) = &self.internal.player {
            let result = if let Err(_) = player.next() {
                let offset = Duration::from_secs(999999);
                player.seek_forwards(&offset).is_ok()
            } else {
                true
            };

            if result {
                self.internal.cached_position = None;
                std::thread::sleep(Duration::from_millis(100));
                self.refresh_player();
                return true;
            }
        }
        false
    }

    pub fn previous_track(&mut self) -> bool {
        if let Some(player) = &self.internal.player {
            let result = if let Err(_) = player.previous() {
                let offset = Duration::from_secs(999999);
                player.seek_backwards(&offset).is_ok()
            } else {
                true
            };

            if result {
                self.internal.cached_position = None;
                std::thread::sleep(Duration::from_millis(100));
                self.refresh_player();
                return true;
            }
        }
        false
    }

    pub fn seek_to(&mut self, position_seconds: f32) -> bool {
        if let Some(player) = &self.internal.player {
            let position_micros = (position_seconds * 1_000_000.0) as i64;
            
            if let Ok(metadata) = player.get_metadata() {
                if let Some(track_id) = metadata.track_id() {
                    let position = Duration::from_micros(position_micros as u64);
                    if player.set_position(track_id, &position).is_ok() {
                        self.state.current_time = position_seconds;
                        self.internal.cached_position = Some(position_micros);
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self::new()
    }
}
