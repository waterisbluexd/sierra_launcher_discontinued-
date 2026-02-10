mod utils;
mod config;
mod panels;
mod app;

use app::state::{Launcher, Panel};
use app::message::Message;
use iced_layershell::application;
use iced::{Task as Command, Color};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity};
use iced_layershell::settings::{LayerShellSettings, Settings};
use crate::utils::theme::Theme;
use crate::utils::wallpaper_manager::{WallpaperManager, WallpaperIndex};
use crate::config::Config;
use crate::panels::search_bar::SearchBar;
use crate::panels::app_list::AppList;
use crate::panels::mpris_player::MusicPlayer;
use crate::panels::system::SystemPanel;
use crate::panels::services::ServicesPanel;
use crate::panels::weather::WeatherPanel;
use crate::panels::title_color::TitleAnimator;
use std::time::Instant;
use std::thread;

fn main() -> Result<(), iced_layershell::Error> {
    let start = Instant::now();
    
    application(new, namespace, update, view)
        .settings(Settings {
            layer_settings: LayerShellSettings {
                size: Some((484, 714)),
                anchor: Anchor::Bottom,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                margin: (0, 0, 4, 0),
                ..Default::default()
            },
            ..Default::default()
        })
        .style(|_theme, _id| iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        })
        .subscription(|_| app::subscription::subscription())
        .run()?;
    
    eprintln!("[Main] Total runtime: {:?}", start.elapsed());
    Ok(())
}

fn new() -> (Launcher, Command<Message>) {
    let start = Instant::now();

    // NON-BLOCKING: Clipboard init in background
    thread::spawn(|| {
        crate::utils::data::init();
    });

    let config = Config::load();
    let mut wallpaper_selected_index = 0;

    let wallpaper_index: Option<WallpaperIndex> =
        if let Some(wallpaper_dir) = config.wallpaper_dir.clone() {
            let manager = WallpaperManager::new(wallpaper_dir.clone());
            
            // ✅ CRITICAL FIX: Restore last wallpaper IMMEDIATELY on startup
            let manager_restore = WallpaperManager::new(wallpaper_dir.clone());
            thread::spawn(move || {
                eprintln!("[Main] Restoring last wallpaper...");
                manager_restore.restore_last_wallpaper();
            });
            
            // Fast synchronous load of index (just reading JSON)
            let index = manager.load_index();
            
            // If no cache, generate in background (non-blocking)
            if index.is_none() {
                let manager_bg = WallpaperManager::new(wallpaper_dir.clone());
                thread::spawn(move || {
                    manager_bg.ensure_cache();
                });
            }

            // Restore selected index from last wallpaper
            if let (Some(last_wallpaper_path), Some(idx)) = (manager.get_last_wallpaper(), &index) {
                if let Some(pos) = idx.wallpapers.iter().position(|e| e.path == last_wallpaper_path) {
                    wallpaper_selected_index = pos;
                }
            }
            index
        } else {
            None
        };

    let theme = Theme::load_from_config(&config);
    
    // NON-BLOCKING: Start clipboard monitor in background
    let _clipboard_monitor = crate::utils::monitor::start_monitor();

    let search_bar = SearchBar::new();
    let app_list = AppList::new();  // Empty initially, loads lazily
    let weather_panel = WeatherPanel::new();  // Already async
    let music_player = MusicPlayer::new();
    let system_panel = SystemPanel::new();
    let services_panel = ServicesPanel::new();
    let title_animator = TitleAnimator::new()
        .with_mode(config.get_animation_mode())
        .with_speed(80);

    eprintln!("[Main] Init: {:?}", start.elapsed());

    (
        Launcher {
            theme,
            watcher: None,  // Initialize lazily on first frame
            config,
            search_bar,
            app_list,
            current_panel: Panel::Clock,
            weather_panel,
            music_player,
            system_panel,
            services_panel,
            last_color_check: Instant::now(),
            last_services_refresh: Instant::now(),
            frame_count: 0,
            title_animator,
            control_center_visible: false,
            clipboard_visible: false,
            clipboard_selected_index: 0,
            is_first_frame: true,
            wallpaper_index,
            wallpaper_selected_index,
        },
        Command::none(),
    )
}

fn namespace() -> String {
    String::from("iced_launcher2")
}

fn update(launcher: &mut Launcher, message: Message) -> Command<Message> {
    app::update::update(launcher, message)
}

fn view(launcher: &Launcher) -> iced::Element<'_, Message> {
    app::view::view(launcher)
}