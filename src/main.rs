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
use crate::utils::wallpaper_manager::WallpaperManager;  // Removed WallpaperIndex
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

    // ═══════════════════════════════════════════════════════════
    // ULTRA-FAST PATH: Only absolute essentials before UI render
    // ═══════════════════════════════════════════════════════════

    // Config is fast (10ms) - keep synchronous
    let config = Config::load();
    eprintln!("[Perf] Config loaded: {:?}", start.elapsed());

    // Theme is fast (15ms) - keep synchronous  
    let theme = Theme::load_from_config(&config);
    eprintln!("[Perf] Theme loaded: {:?}", start.elapsed());

    // ═══════════════════════════════════════════════════════════
    // EVERYTHING ELSE: Background threads (non-blocking)
    // ═══════════════════════════════════════════════════════════

    // 1. Clipboard init (background)
    thread::spawn(|| {
        let t = Instant::now();
        crate::utils::data::init();
        eprintln!("[Background] Clipboard init: {:?}", t.elapsed());
    });

    // 2. Clipboard monitor (background)
    thread::spawn(|| {
        let t = Instant::now();
        let _monitor = crate::utils::monitor::start_monitor();
        eprintln!("[Background] Clipboard monitor: {:?}", t.elapsed());
        // Keep monitor alive
        loop { std::thread::park(); }
    });

    // 3. Wallpaper restoration (background - immediate)
    if let Some(ref wallpaper_dir) = config.wallpaper_dir {
        let wp_dir = wallpaper_dir.clone();
        thread::spawn(move || {
            let t = Instant::now();
            let manager = WallpaperManager::new(wp_dir);
            manager.restore_last_wallpaper();
            eprintln!("[Background] Wallpaper restored: {:?}", t.elapsed());
        });
    }

    // 4. Wallpaper index loading (background)
    let wallpaper_dir_clone = config.wallpaper_dir.clone();
    
    if let Some(wp_dir) = wallpaper_dir_clone {
        thread::spawn(move || {
            let t = Instant::now();
            let manager = WallpaperManager::new(wp_dir.clone());
            
            // Load or generate index
            if manager.load_index().is_none() {
                eprintln!("[Background] Generating wallpaper cache...");
                manager.ensure_cache();
            }
            
            eprintln!("[Background] Wallpaper index ready: {:?}", t.elapsed());
        });
    }

    // ═══════════════════════════════════════════════════════════
    // UI Components: Create with minimal overhead
    // ═══════════════════════════════════════════════════════════

    let search_bar = SearchBar::new();
    let app_list = AppList::new();  // Empty, loads lazily on first frame
    let weather_panel = WeatherPanel::new();  // Already async
    let music_player = MusicPlayer::new();
    let system_panel = SystemPanel::new();
    let services_panel = ServicesPanel::new();
    let title_animator = TitleAnimator::new()
        .with_mode(config.get_animation_mode())
        .with_speed(80);

    eprintln!("[Perf] Total init time: {:?}", start.elapsed());

    (
        Launcher {
            theme,
            watcher: None,  // Initialize on first frame if needed
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
            wallpaper_index: None,  
            wallpaper_selected_index: 0,
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