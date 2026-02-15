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
use iced_layershell::settings::{Settings, LayerShellSettings, StartMode};
use iced::window::Id;
use crate::utils::theme::Theme;
use crate::utils::wallpaper_manager::WallpaperManager;
use crate::config::Config;
use crate::panels::search_bar::SearchBar;
use crate::panels::app_list::{AppList, self};
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
    
    if ipc::is_daemon_running() {
        eprintln!("[Main] Daemon already running, sending SHOW command");
        if let Err(e) = ipc::send_command(ipc::IpcCommand::Show) {
            eprintln!("[Main] Failed to send command: {}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }
    
    eprintln!("[Main] Starting daemon mode...");
    
    let ipc_listener = match ipc::create_server() {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("[Main] Failed to create IPC server: {}", e);
            std::process::exit(1);
        }
    };
    
    thread::spawn(move || {
        ipc::listen_for_commands(ipc_listener, |cmd| {
            eprintln!("[IPC] Received {:?}", cmd);
            ipc::store_command(cmd);
        });
    });
    
    let config = Config::load();
    
    crate::utils::theme::preload_theme(&config);
    
    thread::spawn(|| {
        let t = Instant::now();
        crate::panels::app_list::prewarm_cache();
        eprintln!("[Background] App cache pre-warmed: {:?}", t.elapsed());
    });
    
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
            let manager = WallpaperManager::new(wp_dir.clone());
            manager.restore_last_wallpaper();
            eprintln!("[Background] Wallpaper restored: {:?}", t.elapsed());
            
            let t2 = Instant::now();
            crate::utils::wallpaper_manager::preload_wallpaper_index(wp_dir);
            eprintln!("[Background] Wallpaper index pre-loaded: {:?}", t2.elapsed());
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
            size: None,
            anchor: Anchor::Bottom,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            margin: (0, 0, 4, 0),
            start_mode: StartMode::Background,
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

struct DaemonState {
    config: Config,
    windows: HashMap<Id, Launcher>,
    cached_launcher: Option<Launcher>,
}

impl DaemonState {
    fn boot() -> (Self, Command<Message>) {
        let config = Config::load();
        
        (
            Self {
                config,
                windows: HashMap::new(),
                cached_launcher: None,
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
                if let Some(launcher) = self.windows.remove(&id) {
                    let mut cached = launcher;
                    cached.search_bar.input_value.clear();
                    cached.clipboard_visible = false;
                    cached.control_center_visible = false;
                    cached.is_first_frame = true;
                    cached.current_panel = Panel::Clock;
                    cached.clipboard_selected_index = 0;
                    self.cached_launcher = Some(cached);
                }
                Command::none()
            }
            
            Message::ShowWindow => {
                eprintln!("[Daemon] ShowWindow - creating new window");
                
                if !self.windows.is_empty() {
                    eprintln!("[Daemon] Window already exists, skipping creation");
                    return Command::none();
                }
                
                let id = Id::unique();
                let launcher = self.cached_launcher.take().unwrap_or_else(|| self.create_launcher());
                self.windows.insert(id, launcher);
                
                Command::done(Message::NewLayerShell {
                    settings: NewLayerShellSettings {
                        size: Some((WINDOW_WIDTH, WINDOW_HEIGHT)),
                        layer: Layer::Overlay,
                        anchor: Anchor::Bottom,
                        exclusive_zone: Some(0),
                        margin: Some((0, 0, 4, 0)),
                        keyboard_interactivity: KeyboardInteractivity::OnDemand,
                        output_option: OutputOption::None,
                        events_transparent: false,
                        namespace: Some("sierra_launcher".to_string()),
                    },
                    id,
                })
            }
            
            Message::WindowReady => {
                if let Some(launcher) = self.windows.values_mut().next() {
                    eprintln!("[Daemon] Window ready - focusing search bar");
                    return iced::widget::operation::focus(launcher.search_bar.input_id.clone());
                }
                Command::none()
            }
            
            Message::Close(id) => {
                eprintln!("[Daemon] Closing window: {:?}", id);
                self.windows.remove(&id);
                iced::window::close(id)
            }
            
            Message::AppLaunched => {
                if let Some((&id, launcher)) = self.windows.iter_mut().next() {
                    eprintln!("[Daemon] App launched - closing window {:?}", id);
                    launcher.search_bar.input_value.clear();
                    return iced::window::close(id);
                }
                Command::none()
            }
            
            Message::IcedEvent(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { 
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape), 
                .. 
            })) => {
                if let Some((&id, launcher)) = self.windows.iter_mut().next() {
                    eprintln!("[Input] ESC pressed - closing window {:?}", id);
                    launcher.search_bar.input_value.clear();
                    launcher.clipboard_visible = false;
                    launcher.control_center_visible = false;
                    return iced::window::close(id);
                }
                Command::none()
            }
            
            Message::IcedEvent(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { 
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter), 
                .. 
            })) => {
                if let Some((id, launcher)) = self.windows.iter_mut().next() {
                    eprintln!("[Input] Enter pressed - launching app");
                    let _ = launcher.app_list.update(panels::app_list::Message::LaunchSelected);
                    launcher.search_bar.input_value.clear();
                    return iced::window::close(*id);
                }
                Command::none()
            }
            
            Message::IcedEvent(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { 
                key: iced::keyboard::Key::Named(named), 
                modifiers,
                ..
            })) => {
                eprintln!("[Input] Key pressed: {:?}, modifiers: {:?}", named, modifiers);
                if let Some(launcher) = self.windows.values_mut().next() {
                    match named {
                        iced::keyboard::key::Named::ArrowLeft => {
                            if modifiers.shift() {
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            } else {
                                eprintln!("[Input] Left - cycling panel left");
                                launcher.current_panel = match launcher.current_panel {
                                    Panel::Clock => Panel::System,
                                    Panel::System => Panel::Services,
                                    Panel::Services => Panel::Wallpaper,
                                    Panel::Wallpaper => Panel::Music,
                                    Panel::Music => Panel::Weather,
                                    Panel::Weather => Panel::Clock,
                                };
                            }
                        }
                        iced::keyboard::key::Named::ArrowRight => {
                            if modifiers.shift() {
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            } else {
                                eprintln!("[Input] Right - cycling panel right");
                                launcher.current_panel = match launcher.current_panel {
                                    Panel::Clock => Panel::Weather,
                                    Panel::Weather => Panel::Music,
                                    Panel::Music => Panel::Wallpaper,
                                    Panel::Wallpaper => Panel::Services,
                                    Panel::Services => Panel::System,
                                    Panel::System => Panel::Clock,
                                };
                            }
                        }
                        iced::keyboard::key::Named::ArrowUp => {
                            if launcher.clipboard_visible {
                                if launcher.clipboard_selected_index > 0 {
                                    launcher.clipboard_selected_index -= 1;
                                }
                            } else {
                                let _ = launcher.app_list.update(panels::app_list::Message::ArrowUp);
                            }
                        }
                        iced::keyboard::key::Named::ArrowDown => {
                            if launcher.clipboard_visible {
                                launcher.clipboard_selected_index += 1;
                            } else {
                                let _ = launcher.app_list.update(panels::app_list::Message::ArrowDown);
                            }
                        }
                        _ => {}
                    }
                }
                Command::none()
            }
            
            Message::IcedEvent(iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { 
                key: iced::keyboard::Key::Character(c), 
                modifiers,
                ..
            })) if modifiers.control() && c.as_str() == "d" => {
                if let Some(launcher) = self.windows.values_mut().next() {
                    if launcher.clipboard_visible {
                        eprintln!("[Input] Ctrl+D - deleting clipboard entry");
                        let items = crate::utils::data::search_items("");
                        if !items.is_empty() && launcher.clipboard_selected_index < items.len() {
                            crate::utils::data::delete_item(launcher.clipboard_selected_index);
                            let new_count = crate::utils::data::item_count();
                            if launcher.clipboard_selected_index >= new_count && new_count > 0 {
                                launcher.clipboard_selected_index = new_count - 1;
                            } else if new_count == 0 {
                                launcher.clipboard_selected_index = 0;
                            }
                        }
                    }
                }
                Command::none()
            }
            
            Message::IcedEvent(_) => Command::none(),
            
            other => {
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
        let ipc_poll = iced::time::every(std::time::Duration::from_millis(16))
            .filter_map(|_| {
                if ipc::poll_show() {
                    Some(Message::ShowWindow)
                } else {
                    None
                }
            });
        
        let close_events = iced::window::close_events().map(Message::WindowClosed);
        let events = iced::event::listen().map(Message::IcedEvent);
        
        let color_check = iced::time::every(std::time::Duration::from_millis(500))
            .map(|_| Message::CheckColors);
        
        let music_refresh = iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::MusicRefresh);
        
        iced::Subscription::batch(vec![ipc_poll, close_events, events, color_check, music_refresh])
    }
    
    fn create_launcher(&self) -> Launcher {
        Self::create_launcher_template(&self.config)
    }
    
    fn create_launcher_template(config: &Config) -> Launcher {
        let theme = Theme::load_from_config(config);
        
        Launcher {
            theme,
            watcher: None,
            config: config.clone(),
            search_bar: SearchBar::new(),
            app_list: AppList::new(),
            current_panel: Panel::Clock,
            weather_panel: WeatherPanel::with_location(config.weather_location.clone()),
            music_player: MusicPlayer::new(),
            system_panel: SystemPanel::new(),
            services_panel: ServicesPanel::new(),
            last_color_check: Instant::now(),
            last_services_refresh: Instant::now(),
            last_pywal_reload: Instant::now(),
            frame_count: 0,
            title_animator: TitleAnimator::new()
                .with_mode(config.get_animation_mode())
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
