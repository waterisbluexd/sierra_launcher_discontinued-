mod utils;
mod config;
mod panels;
mod app;
mod ipc;

use app::state::{Launcher, Panel, PopupState};
use app::message::Message;
use iced_layershell::build_pattern::daemon;
use iced::{Task as Command, Color, Element};
use iced_layershell::reexport::{KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption};
use iced_layershell::settings::{Settings, LayerShellSettings, StartMode};
use iced::window::Id;

pub use iced_layershell::reexport::Anchor;
use crate::utils::theme::Theme;
use crate::utils::wallpaper_manager::WallpaperManager;
use crate::config::Config;
use crate::panels::main::search_bar::SearchBar;
use crate::panels::main::app_list::{AppList, self};
use crate::panels::media::mpris_player::MusicPlayer;
use crate::panels::system::system::SystemPanel;
use crate::panels::system::services::ServicesPanel;
use crate::panels::system::wifi_panel::WifiPanel;
use crate::panels::weather::WeatherPanel;
use crate::panels::title_color::TitleAnimator;
use app::message::{WINDOW_WIDTH, WINDOW_HEIGHT, POPUP_HEIGHT, POPUP_GAP};
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
        crate::panels::main::app_list::prewarm_cache();
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
            anchor: config.get_anchor(),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            margin: config.get_margin(),
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
    popup_launcher: Option<Launcher>,
}

impl DaemonState {
    fn boot() -> (Self, Command<Message>) {
        let config = Config::load();
        
        (
            Self {
                config,
                windows: HashMap::new(),
                cached_launcher: None,
                popup_launcher: None,
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
                
                // Check if this is the popup window closing
                let is_popup = self.windows.values().any(|launcher| {
                    launcher.popup_state.window_id == Some(id)
                });
                
                if is_popup {
                    eprintln!("[Daemon] Popup window closed, cleaning up popup state");
                    if let Some(launcher) = self.windows.values_mut().next() {
                        launcher.popup_state.window_id = None;
                        launcher.popup_state.visible = false;
                        launcher.popup_state.hover_active = false;
                        launcher.popup_state.close_timer = None;
                    }
                    self.popup_launcher = None;
                    return Command::none();
                }
                
                // Main window closing
                if let Some(launcher) = self.windows.remove(&id) {
                    let mut cached = launcher;
                    cached.search_bar.input_value.clear();
                    cached.clipboard_visible = false;
                    cached.control_center_visible = false;
                    cached.is_first_frame = true;
                    cached.current_panel = Panel::Clock;
                    cached.clipboard_selected_index = 0;
                    // Clean up any popup state
                    cached.popup_state = PopupState::default();
                    self.cached_launcher = Some(cached);
                }
                self.popup_launcher = None;
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
                        anchor: self.config.get_anchor(),
                        exclusive_zone: Some(0),
                        margin: Some(self.config.get_margin()),
                        keyboard_interactivity: KeyboardInteractivity::OnDemand,
                        output_option: OutputOption::None,
                        events_transparent: false,
                        namespace: Some("sierra_launcher".to_string()),
                    },
                    id,
                })
            }
            
            Message::CreatePopupWindow => {
                eprintln!("[Daemon] CreatePopupWindow - creating popup window as layer shell");
                
                if let Some(launcher) = self.windows.values_mut().next() {
                    if launcher.popup_state.window_id.is_some() {
                        eprintln!("[Daemon] Popup window already exists");
                        return Command::none();
                    }
                    
                    let popup_id = Id::unique();
                    launcher.popup_state.window_id = Some(popup_id);
                    
                    // Create popup launcher with same config
                    let popup_launcher = Self::create_launcher_template(&self.config);
                    self.popup_launcher = Some(popup_launcher);
                    
                    // Use NewLayerShell so the popup receives mouse events properly.
                    // xdg-popup (NewPopUp) does not reliably receive input on wlroots compositors.
                    return Command::done(Message::NewLayerShell {
                        settings: NewLayerShellSettings {
                            size: Some((WINDOW_WIDTH, POPUP_HEIGHT)),
                            layer: Layer::Overlay,
                            anchor: self.config.get_anchor(),
                            exclusive_zone: Some(0),
                            // Position it relative to the main window based on config
                            margin: Some(self.config.get_popup_margin()),
                            keyboard_interactivity: KeyboardInteractivity::OnDemand,
                            output_option: OutputOption::None,
                            events_transparent: false,  // MUST be false to receive mouse events
                            namespace: Some("sierra_launcher_popup".to_string()),
                        },
                        id: popup_id,
                    });
                }
                Command::none()
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
            
            Message::PopupTick => {
                if let Some(launcher) = self.windows.values_mut().next() {
                    if launcher.popup_state.visible && !launcher.popup_state.hover_active {
                        // Start close timer if not already started
                        if launcher.popup_state.close_timer.is_none() {
                            launcher.popup_state.close_timer = Some(Instant::now());
                        } else if let Some(timer_start) = launcher.popup_state.close_timer {
                            // Check if 1000ms has passed (give time for hover enter to cancel)
                            if timer_start.elapsed().as_millis() > 1000 {
                                eprintln!("[Popup] Auto-closing popup after timeout");
                                launcher.popup_state.visible = false;
                                launcher.popup_state.close_timer = None;
                                
                                // Close the popup window
                                if let Some(popup_id) = launcher.popup_state.window_id {
                                    launcher.popup_state.window_id = None;
                                    return iced::window::close(popup_id);
                                }
                            }
                        }
                    } else if launcher.popup_state.visible && launcher.popup_state.hover_active {
                        // Reset timer when hovering
                        launcher.popup_state.close_timer = None;
                    }
                }
                Command::none()
            }
            
            Message::PopupHoverEnter => {
                eprintln!("[Popup] >>> PopupHoverEnter FIRED <<< (popup is receiving events!)");
                if let Some(launcher) = self.windows.values_mut().next() {
                    launcher.popup_state.hover_active = true;
                    launcher.popup_state.close_timer = None;  // KEY FIX: cancel close timer
                    eprintln!("[Popup] hover_active=true, close_timer=None");
                }
                Command::none()
            }
            
            Message::PopupHoverExit => {
                eprintln!("[Popup] >>> PopupHoverExit FIRED <<<");
                if let Some(launcher) = self.windows.values_mut().next() {
                    launcher.popup_state.hover_active = false;
                    launcher.popup_state.close_timer = Some(Instant::now());
                    eprintln!("[Popup] hover_active=false, close_timer started");
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
                    let _ = launcher.app_list.update(panels::main::app_list::Message::LaunchSelected);
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
                                let music_available = launcher.music_player.state.player_available;
                                launcher.current_panel = match launcher.current_panel {
                                    Panel::Clock => Panel::System,
                                    Panel::System => Panel::Services,
                                    Panel::Services => Panel::Wallpaper,
                                    Panel::Wallpaper => {
                                        if music_available { Panel::Music } else { Panel::Weather }
                                    }
                                    Panel::Music => Panel::Weather,
                                    Panel::Weather => Panel::Clock,
                                    Panel::Wifi => Panel::Clock,
                                };
                                // Skip Music if not available
                                if launcher.current_panel == Panel::Music && !music_available {
                                    launcher.current_panel = Panel::Weather;
                                }
                            }
                        }
                        iced::keyboard::key::Named::ArrowRight => {
                            if modifiers.shift() {
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            } else {
                                eprintln!("[Input] Right - cycling panel right");
                                let music_available = launcher.music_player.state.player_available;
                                launcher.current_panel = match launcher.current_panel {
                                    Panel::Clock => Panel::Weather,
                                    Panel::Weather => {
                                        if music_available { Panel::Music } else { Panel::Wallpaper }
                                    }
                                    Panel::Music => Panel::Wallpaper,
                                    Panel::Wallpaper => Panel::Services,
                                    Panel::Services => Panel::System,
                                    Panel::System => Panel::Clock,
                                    Panel::Wifi => Panel::Clock,
                                };
                            }
                        }
                        iced::keyboard::key::Named::ArrowUp => {
                            if launcher.clipboard_visible {
                                if launcher.clipboard_selected_index > 0 {
                                    launcher.clipboard_selected_index -= 1;
                                }
                            } else {
                                let _ = launcher.app_list.update(panels::main::app_list::Message::ArrowUp);
                            }
                        }
                        iced::keyboard::key::Named::ArrowDown => {
                            if launcher.clipboard_visible {
                                launcher.clipboard_selected_index += 1;
                            } else {
                                let _ = launcher.app_list.update(panels::main::app_list::Message::ArrowDown);
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
            
            Message::IcedEvent(iced::Event::Mouse(mouse_event)) => {
                if let Some(launcher) = self.windows.values_mut().next() {
                    match mouse_event {
                        iced::mouse::Event::CursorMoved { position, .. } => {
                            let y = position.y;
                            
                            // KEY FIX: When popup exists, CursorMoved events with small y values
                            // (0..POPUP_HEIGHT) are from the popup window's coordinate space, not main window.
                            // When popup exists, rely ONLY on PopupHoverEnter/Exit for visibility management.
                            let popup_exists = launcher.popup_state.window_id.is_some();
                            
                            if popup_exists {
                                // Popup exists - only use CursorMoved to detect when mouse is
                                // clearly back in main window area (y > POPUP_HEIGHT + margin)
                                // to start close timer if not hovering popup
                                if y > 150.0 
                                    && launcher.popup_state.visible 
                                    && !launcher.popup_state.hover_active 
                                {
                                    if launcher.popup_state.close_timer.is_none() {
                                        eprintln!("[Popup] Mouse clearly in main window (y={:.0}), start close timer", y);
                                        launcher.popup_state.close_timer = Some(Instant::now());
                                    }
                                }
                                launcher.popup_state.last_mouse_y = y;
                                return Command::none();
                            }
                            
                            // No popup exists - normal behavior: show popup when mouse near top
                            let threshold = 80.0;
                            
                            if y < threshold && !launcher.popup_state.visible {
                                eprintln!("[Popup] Mouse at top (y={:.1}), showing popup", y);
                                launcher.popup_state.visible = true;
                                launcher.popup_state.close_timer = None;
                                launcher.popup_state.hover_active = true;
                                
                                if launcher.popup_state.window_id.is_none() {
                                    eprintln!("[Popup] Requesting popup window creation");
                                    return Command::done(Message::CreatePopupWindow);
                                }
                            }
                            
                            launcher.popup_state.last_mouse_y = y;
                        }
                        _ => {}
                    }
                }
                Command::none()
            }
            
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
        // Check if this is the popup window
        if let Some(popup_launcher) = &self.popup_launcher {
            // Check if any main window has this popup_id
            for (_, main_launcher) in &self.windows {
                if let Some(popup_id) = main_launcher.popup_state.window_id {
                    if id == popup_id {
                        // Use main launcher's current_workspace for real-time updates
                        return app::view::popup_view_with_workspace(
                            popup_launcher, 
                            main_launcher.current_workspace,
                        );
                    }
                }
            }
        }
        
        // Regular main window
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
        
        let popup_tick = iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::PopupTick);
        
        let workspace_refresh = iced::time::every(std::time::Duration::from_millis(500))
            .map(|_| Message::RefreshWorkspace);
        
        iced::Subscription::batch(vec![ipc_poll, close_events, events, color_check, music_refresh, popup_tick, workspace_refresh])
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
            wifi_panel: WifiPanel::new(),
            last_color_check: Instant::now(),
            last_services_refresh: Instant::now(),
            last_pywal_reload: Instant::now(),
            frame_count: 0,
            title_animator: TitleAnimator::new()
                .with_mode(config.get_animation_mode())
                .with_speed(config.title_animation_speed as u64),
            control_center_visible: false,
            clipboard_visible: false,
            clipboard_selected_index: 0,
            is_first_frame: true,
            wallpaper_index: None,
            wallpaper_selected_index: 0,
            popup_state: PopupState::new(),
            current_workspace: crate::panels::workspace::current_window_manager::get_current_workspace(),
        }
    }
}
