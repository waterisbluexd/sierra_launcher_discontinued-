mod utils;
mod config;
mod panels;
mod app;
mod ipc;

use app::state::{Launcher, Panel};
use app::message::Message;
use iced_layershell::build_pattern::daemon;
use iced::{Task as Command, Color, Element};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};
use iced_layershell::settings::{Settings, LayerShellSettings};
use iced::window::Id;
use crate::utils::theme::Theme;
use crate::utils::wallpaper_manager::WallpaperManager;
use crate::config::Config;
use crate::panels::search_bar::SearchBar;
use crate::panels::app_list::AppList;
use crate::panels::mpris_player::MusicPlayer;
use crate::panels::system::SystemPanel;
use crate::panels::services::ServicesPanel;
use crate::panels::weather::WeatherPanel;
use crate::panels::title_color::TitleAnimator;
use app::message::{WINDOW_WIDTH, WINDOW_HEIGHT};
use std::time::Instant;
use std::thread;
use std::collections::HashMap;

fn main() -> Result<(), iced_layershell::Error> {
    let start = Instant::now();
    
    // Check if daemon is already running
    if ipc::is_daemon_running() {
        eprintln!("[Main] Daemon already running, sending SHOW command");
        if let Err(e) = ipc::send_command(ipc::IpcCommand::Show) {
            eprintln!("[Main] Failed to send command: {}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }
    
    eprintln!("[Main] Starting daemon mode...");
    
    // Create IPC server
    let ipc_listener = match ipc::create_server() {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("[Main] Failed to create IPC server: {}", e);
            std::process::exit(1);
        }
    };
    
    // Start IPC listener thread
    thread::spawn(move || {
        ipc::listen_for_commands(ipc_listener, |cmd| {
            eprintln!("[IPC] Received {:?}", cmd);
            ipc::store_command(cmd);
        });
    });
    
    // Initialize background services
    let config = Config::load();
    let _theme = Theme::load_from_config(&config);
    
    thread::spawn(|| {
        let t = Instant::now();
        crate::utils::data::init();
        eprintln!("[Background] Clipboard init: {:?}", t.elapsed());
    });

    thread::spawn(|| {
        let t = Instant::now();
        let _monitor = crate::utils::monitor::start_monitor();
        eprintln!("[Background] Clipboard monitor: {:?}", t.elapsed());
        loop { std::thread::park(); }
    });

    if let Some(ref wallpaper_dir) = config.wallpaper_dir {
        let wp_dir = wallpaper_dir.clone();
        thread::spawn(move || {
            let t = Instant::now();
            let manager = WallpaperManager::new(wp_dir);
            manager.restore_last_wallpaper();
            eprintln!("[Background] Wallpaper restored: {:?}", t.elapsed());
        });
    }
    
    daemon(
        DaemonState::boot,
        DaemonState::namespace,
        DaemonState::update,
        DaemonState::view,
    )
    .subscription(DaemonState::subscription)
    .settings(Settings {
        layer_settings: LayerShellSettings {
            size: Some((WINDOW_WIDTH, WINDOW_HEIGHT)),
            anchor: Anchor::Bottom,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            margin: (0, 0, 4, 0),
            ..Default::default()
        },
        ..Default::default()
    })
    .style(|_theme, _id| iced::theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: Color::WHITE,
    })
    .run()?;
    
    utils::data::save_on_shutdown();
    eprintln!("[Main] Total runtime: {:?}", start.elapsed());
    Ok(())
}

/// Daemon state managing multiple windows
struct DaemonState {
    /// Configuration
    config: Config,
    /// Active launcher windows
    windows: HashMap<Id, Launcher>,
}

impl DaemonState {
    fn boot() -> (Self, Command<Message>) {
        let config = Config::load();
        
        (
            Self {
                config,
                windows: HashMap::new(),
            },
            Command::none(),
        )
    }
    
    fn namespace() -> String {
        String::from("sierra_launcher")
    }
    
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::WindowClosed(id) => {
                eprintln!("[Daemon] Window closed: {:?}", id);
                self.windows.remove(&id);
                Command::none()
            }
            
            Message::ShowWindow => {
                eprintln!("[Daemon] ShowWindow - creating new window");
                
                // Create a new launcher window
                let id = Id::unique();
                let launcher = self.create_launcher();
                self.windows.insert(id, launcher);
                
                // Create new layer shell with exclusive keyboard
                // Use OutputOption::None to show on the current active output (where mouse is)
                Command::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((WINDOW_WIDTH, WINDOW_HEIGHT)),
                        layer: Layer::Overlay,
                        anchor: Anchor::Bottom,
                        exclusive_zone: Some(-1),
                        margin: Some((0, 0, 4, 0)),
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        output_option: OutputOption::None,
                        events_transparent: false,
                        namespace: Some("sierra_launcher".to_string()),
                    },
                    id,
                })
            }
            
            Message::Close(id) => {
                eprintln!("[Daemon] Closing window: {:?}", id);
                self.windows.remove(&id);
                // Use iced's built-in window close
                iced::window::close(id)
            }
            
            Message::IcedEvent(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { 
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape), 
                .. 
            })) => {
                // Close the first active window on ESC
                if let Some((&id, launcher)) = self.windows.iter_mut().next() {
                    eprintln!("[Input] ESC pressed - closing window {:?}", id);
                    launcher.search_bar.input_value.clear();
                    launcher.clipboard_visible = false;
                    launcher.control_center_visible = false;
                    return iced::window::close(id);
                }
                Command::none()
            }
            
            Message::IcedEvent(_) => Command::none(),
            
            // Route other messages to active windows
            other => {
                // Send to all active windows
                let mut commands = Vec::new();
                for launcher in self.windows.values_mut() {
                    commands.push(app::update::update(launcher, other.clone()));
                }
                Command::batch(commands)
            }
        }
    }
    
    fn view(&self, id: Id) -> Element<'_, Message> {
        if let Some(launcher) = self.windows.get(&id) {
            app::view::view(launcher)
        } else {
            iced::widget::text("").into()
        }
    }
    
    fn subscription(&self) -> iced::Subscription<Message> {
        // Poll for IPC commands
        let ipc_poll = iced::window::frames()
            .filter_map(|_| {
                if ipc::poll_show() {
                    Some(Message::ShowWindow)
                } else {
                    None
                }
            });
        
        // Listen for window close events
        let close_events = iced::window::close_events().map(Message::WindowClosed);
        
        // Listen for keyboard events
        let events = iced::event::listen().map(Message::IcedEvent);
        
        // Frame-based checks for active windows
        let frames = iced::window::frames().map(|_| Message::CheckColors);
        let music_refresh = iced::window::frames().map(|_| Message::MusicRefresh);
        
        iced::Subscription::batch(vec![ipc_poll, close_events, events, frames, music_refresh])
    }
    
    fn create_launcher(&self) -> Launcher {
        let theme = Theme::load_from_config(&self.config);
        
        Launcher {
            theme,
            watcher: None,
            config: self.config.clone(),
            search_bar: SearchBar::new(),
            app_list: AppList::new(),
            current_panel: Panel::Clock,
            weather_panel: WeatherPanel::new(),
            music_player: MusicPlayer::new(),
            system_panel: SystemPanel::new(),
            services_panel: ServicesPanel::new(),
            last_color_check: Instant::now(),
            last_services_refresh: Instant::now(),
            last_pywal_reload: Instant::now(),
            frame_count: 0,
            title_animator: TitleAnimator::new()
                .with_mode(self.config.get_animation_mode())
                .with_speed(80),
            control_center_visible: false,
            clipboard_visible: false,
            clipboard_selected_index: 0,
            is_first_frame: true,
            wallpaper_index: None,
            wallpaper_selected_index: 0,
        }
    }
}
