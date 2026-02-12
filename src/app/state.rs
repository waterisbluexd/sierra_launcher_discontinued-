
use crate::panels::title_color::TitleAnimator;
use crate::utils::theme::Theme;
use crate::utils::watcher::ColorWatcher;
use crate::utils::wallpaper_manager::WallpaperIndex;
use crate::config::Config;
use crate::panels::search_bar::SearchBar;
use crate::panels::app_list::AppList;
use crate::panels::mpris_player::MusicPlayer;
use crate::panels::system::SystemPanel;
use crate::panels::services::ServicesPanel;
use crate::panels::weather::WeatherPanel;

use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Clock,
    Weather,
    Music,
    Wallpaper,
    System,
    Services,
}

pub struct Launcher {
    pub theme: Theme,
    pub watcher: Option<ColorWatcher>,
    pub config: Config,
    pub search_bar: SearchBar,
    pub app_list: AppList,
    pub current_panel: Panel,
    pub weather_panel: WeatherPanel,
    pub music_player: MusicPlayer,
    pub system_panel: SystemPanel,
    pub services_panel: ServicesPanel,
    pub last_color_check: Instant,
    pub last_services_refresh: Instant,
    pub last_pywal_reload: Instant,
    pub frame_count: u32,
    pub title_animator: TitleAnimator,
    pub control_center_visible: bool,
    pub clipboard_visible: bool,
    pub clipboard_selected_index: usize,
    pub is_first_frame: bool,
    pub wallpaper_index: Option<WallpaperIndex>,
    pub wallpaper_selected_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}
