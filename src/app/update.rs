use iced::Task as Command;
use iced::Event;
use iced::keyboard;
use keyboard::key::Named;
use iced::mouse;

use crate::app::state::{Launcher, Panel, Direction};
use crate::app::message::Message;
use crate::panels::{search_bar, app_list};
use crate::utils::watcher::ColorWatcher; 
use std::time::{Duration, Instant};

pub fn update(launcher: &mut Launcher, message: Message) -> Command<Message> {
    match message {
        Message::IcedEvent(event) => {
            match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    match key {
                        // ESC is now handled in main.rs daemon mode - it closes the window
                        keyboard::Key::Named(Named::Escape) => {
                            // Window will be closed by daemon, just reset state
                            launcher.search_bar.input_value.clear();
                            let _ = launcher.app_list.update(app_list::Message::SearchInput(String::new()));
                            launcher.clipboard_visible = false;
                            launcher.control_center_visible = false;
                        }
                        
                        keyboard::Key::Named(Named::Enter) => {
                            if launcher.clipboard_visible {
                                return Command::perform(async {}, |_| Message::ClipboardSelect);
                            } else {
                                // Launch selected app
                                let _ = launcher.app_list.update(app_list::Message::LaunchSelected);
                                // DON'T EXIT - window closes itself via daemon
                            }
                        }
                        
                        keyboard::Key::Named(Named::ArrowUp) => {
                            if launcher.clipboard_visible {
                                return Command::perform(async {}, |_| Message::ClipboardArrowUp);
                            } else {
                                let _ = launcher.app_list.update(app_list::Message::ArrowUp);
                            }
                        }
                        
                        keyboard::Key::Named(Named::ArrowDown) => {
                            if launcher.clipboard_visible {
                                return Command::perform(async {}, |_| Message::ClipboardArrowDown);
                            } else {
                                let _ = launcher.app_list.update(app_list::Message::ArrowDown);
                            }
                        }
                        
                        keyboard::Key::Named(Named::ArrowLeft) => {
                            if modifiers.shift() {
                                return Command::perform(async {}, |_| Message::CyclePanel(Direction::Left));
                            } else {
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            }
                        }
                        
                        keyboard::Key::Named(Named::ArrowRight) => {
                            if modifiers.shift() {
                                return Command::perform(async {}, |_| Message::CyclePanel(Direction::Right));
                            } else {
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            }
                        }

                        keyboard::Key::Named(Named::Backspace) => {
                            if !launcher.clipboard_visible && !launcher.search_bar.input_value.is_empty() {
                                launcher.search_bar.input_value.pop();
                                let _ = launcher.app_list.update(app_list::Message::SearchInput(launcher.search_bar.input_value.clone()));
                            }
                        }

                        keyboard::Key::Character(c) => {
                            if launcher.clipboard_visible && modifiers.control() && c.as_str() == "d" {
                                return Command::perform(async {}, |_| Message::ClipboardDelete);
                            } else if !launcher.clipboard_visible && !modifiers.control() && !modifiers.alt() && !modifiers.logo() {
                                launcher.search_bar.input_value.push_str(c.as_str());
                                let _ = launcher.app_list.update(app_list::Message::SearchInput(launcher.search_bar.input_value.clone()));
                            }
                        }
                        
                        _ => {
                            // Ignore other keys
                        }
                    }
                }
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                    launcher.control_center_visible = !launcher.control_center_visible;
                }
                _ => {}
            }

            Command::none()
        }

        Message::CheckColors => {
            if launcher.is_first_frame {
                launcher.is_first_frame = false;
                
                launcher.app_list.start_loading();
                eprintln!("[Main] Triggered lazy app loading");
                launcher.system_panel.start();
                
                // Load wallpaper index synchronously so it's available immediately
                if launcher.wallpaper_index.is_none() {
                    // First try the global pre-loaded cache
                    if let Some(cached_index) = crate::utils::wallpaper_manager::get_cached_index() {
                        launcher.wallpaper_index = Some(cached_index.clone());
                        eprintln!("[Wallpaper] ✓ Using pre-loaded global index");
                        
                        // Set selected index to last wallpaper
                        if let Some(ref wallpaper_dir) = launcher.config.wallpaper_dir {
                            let manager = crate::utils::wallpaper_manager::WallpaperManager::new(wallpaper_dir.clone());
                            if let (Some(last_wallpaper_path), Some(ref idx)) = (manager.get_last_wallpaper(), &launcher.wallpaper_index) {
                                if let Some(pos) = idx.wallpapers.iter().position(|e| e.path == last_wallpaper_path) {
                                    launcher.wallpaper_selected_index = pos;
                                }
                            }
                        }
                    } else if let Some(ref wallpaper_dir) = launcher.config.wallpaper_dir {
                        // Fallback: load on demand if not pre-loaded
                        let manager = crate::utils::wallpaper_manager::WallpaperManager::new(wallpaper_dir.clone());
                        
                        // Generate cache if it doesn't exist
                        manager.ensure_cache();
                        
                        launcher.wallpaper_index = manager.load_index();
                        eprintln!("[Wallpaper] Index loaded: {:?}", launcher.wallpaper_index.as_ref().map(|i| i.wallpapers.len()));
                        
                        // Set selected index to last wallpaper
                        if let (Some(last_wallpaper_path), Some(ref idx)) = (manager.get_last_wallpaper(), &launcher.wallpaper_index) {
                            if let Some(pos) = idx.wallpapers.iter().position(|e| e.path == last_wallpaper_path) {
                                launcher.wallpaper_selected_index = pos;
                            }
                        }
                    }
                }
                
                if launcher.config.use_pywal && launcher.watcher.is_none() {
                    launcher.watcher = ColorWatcher::new().ok();
                }
                
                // Return WindowReady message to trigger focus
                return Command::done(Message::WindowReady);
            }
            
            if launcher.app_list.check_loaded() {
                eprintln!("[Main] Apps finished loading - UI will update automatically");
            }
            
            launcher.frame_count += 1;
            
            launcher.title_animator.update();
            
            let now = Instant::now();
            if now.duration_since(launcher.last_color_check) > Duration::from_secs(2) {
                launcher.last_color_check = now;
                if launcher.config.use_pywal {
                    if let Some(ref watcher) = launcher.watcher {
                        if watcher.check_for_changes() {
                            // Clear cache and reload theme
                            crate::utils::theme::clear_theme_cache();
                            launcher.theme = crate::utils::theme::get_cached_theme(&launcher.config);
                            eprintln!("Pywal theme reloaded");
                        }
                    }
                }
            }
            
            if now.duration_since(launcher.last_services_refresh) > Duration::from_secs(5) {
                launcher.last_services_refresh = now;
                
                if launcher.current_panel == Panel::Services {
                    launcher.services_panel.schedule_refresh();
                }
            }
            
            Command::none()
        }
        
        Message::SearchBarMessage(search_bar_message) => {
            match search_bar_message {
                search_bar::Message::InputChanged(value) => {
                    launcher.search_bar.input_value = value.clone();
                    let _ = launcher.app_list.update(app_list::Message::SearchInput(value));
                    Command::none()
                }
                search_bar::Message::Submitted => {
                    let _ = launcher.app_list.update(app_list::Message::LaunchSelected);
                    // Send AppLaunched message to close the window
                    Command::done(Message::AppLaunched)
                }
            }
        }
        
        Message::AppListMessage(app_list_message) => {
            let _ = launcher.app_list.update(app_list_message);
            Command::none()
        }

        Message::CyclePanel(direction) => {
            launcher.current_panel = match (launcher.current_panel, direction) {
                (Panel::Clock, Direction::Right) => Panel::Weather,
                (Panel::Weather, Direction::Right) => Panel::Music,
                (Panel::Music, Direction::Right) => Panel::Wallpaper,
                (Panel::Wallpaper, Direction::Right) => Panel::System,
                (Panel::System, Direction::Right) => Panel::Services,
                (Panel::Services, Direction::Right) => Panel::Clock,
                (Panel::Clock, Direction::Left) => Panel::Services,
                (Panel::Services, Direction::Left) => Panel::System,
                (Panel::System, Direction::Left) => Panel::Wallpaper,
                (Panel::Wallpaper, Direction::Left) => Panel::Music,
                (Panel::Music, Direction::Left) => Panel::Weather,
                (Panel::Weather, Direction::Left) => Panel::Clock,
            };
            
            if launcher.current_panel == Panel::Services {
                launcher.services_panel.schedule_refresh();
            }
            
            if launcher.current_panel == Panel::Wallpaper && launcher.wallpaper_index.is_none() {
                // First try the global pre-loaded cache
                if let Some(cached_index) = crate::utils::wallpaper_manager::get_cached_index() {
                    launcher.wallpaper_index = Some(cached_index);
                    eprintln!("[Wallpaper] ✓ Using pre-loaded global index (panel switch)");
                } else if let Some(ref wallpaper_dir) = launcher.config.wallpaper_dir {
                    // Fallback: load on demand
                    let manager = crate::utils::wallpaper_manager::WallpaperManager::new(wallpaper_dir.clone());
                    
                    // Generate cache if it doesn't exist
                    manager.ensure_cache();
                    
                    launcher.wallpaper_index = manager.load_index();
                    
                    if let (Some(last_wallpaper_path), Some(ref idx)) = (manager.get_last_wallpaper(), &launcher.wallpaper_index) {
                        if let Some(pos) = idx.wallpapers.iter().position(|e| e.path == last_wallpaper_path) {
                            launcher.wallpaper_selected_index = pos;
                        }
                    }
                }
            }
            
            Command::none()
        }

        Message::MusicPlayPause => {
            launcher.music_player.play_pause();
            Command::none()
        }

        Message::MusicNext => {
            launcher.music_player.next_track();
            Command::none()
        }

        Message::MusicPrevious => {
            launcher.music_player.previous_track();
            Command::none()
        }

        Message::MusicProgressChanged(position) => {
            launcher.music_player.seek_to(position);
            Command::none()
        }

        Message::MusicRefresh => {
            launcher.music_player.refresh_player();
            Command::none()
        }

        Message::VolumeChanged(value) => {
            launcher.services_panel.set_volume(value);
            Command::none()
        }

        Message::BrightnessChanged(value) => {
            launcher.services_panel.set_brightness(value);
            Command::none()
        }

        Message::VolumeMuteToggle => {
            launcher.services_panel.toggle_mute();
            Command::none()
        }
        
        Message::AirplaneModeToggle => {
            launcher.services_panel.toggle_airplane_mode();
            Command::none()
        }
        
        Message::BrightnessMinToggle => {
            launcher.services_panel.toggle_min_brightness();
            Command::none()
        }

        Message::WifiToggle => {
            launcher.services_panel.toggle_wifi();
            Command::none()
        }

        Message::BluetoothToggle => {
            launcher.services_panel.toggle_bluetooth();
            Command::none()
        }

        Message::EyeCareToggle => {
            launcher.services_panel.toggle_eye_care();
            Command::none()
        }

        Message::ToggleControlCenter => {
            launcher.control_center_visible = !launcher.control_center_visible;
            Command::none()
        }

        Message::PowerOffTheSystem => {
            launcher.control_center_visible = false;
            
            // Use double --force for immediate shutdown bypassing session managers
            std::thread::spawn(|| {
                let result = std::process::Command::new("systemctl")
                    .args(["poweroff", "--force", "--force"])
                    .spawn();  // spawn() returns immediately, doesn't wait for completion
                if let Err(e) = result {
                    eprintln!("[PowerOff] Failed: {}", e);
                }
            });
            Command::done(Message::AppLaunched)
        }

        Message::RestartTheSystem => {
            launcher.control_center_visible = false;
            
            // Use double --force for immediate reboot bypassing session managers
            std::thread::spawn(|| {
                let result = std::process::Command::new("systemctl")
                    .args(["reboot", "--force", "--force"])
                    .spawn();  // spawn() returns immediately, doesn't wait for completion
                if let Err(e) = result {
                    eprintln!("[Restart] Failed: {}", e);
                }
            });
            Command::done(Message::AppLaunched)
        }

        Message::SleepModeTheSystem => {
            launcher.control_center_visible = false;
            
            // Use --force for immediate suspend
            std::thread::spawn(|| {
                let result = std::process::Command::new("systemctl")
                    .args(["suspend", "--force"])
                    .spawn();  // spawn() returns immediately, doesn't wait for completion
                if let Err(e) = result {
                    eprintln!("[Suspend] Failed: {}", e);
                }
            });
            Command::done(Message::AppLaunched)
        }

        Message::ClipboardArrowUp => {
            if launcher.clipboard_selected_index > 0 {
                launcher.clipboard_selected_index -= 1;
            }
            Command::none()
        }

        Message::ClipboardArrowDown => {
            let items = crate::utils::data::search_items("");
            if launcher.clipboard_selected_index + 1 < items.len() {
                launcher.clipboard_selected_index += 1;
            }
            Command::none()
        }

        Message::ClipboardSelect => {
                let items = crate::utils::data::search_items("");
                if let Some(item) = items.get(launcher.clipboard_selected_index) {
                    let content = item.full_content();
                
                    crate::utils::monitor::set_ignore_next(content.clone());
                    let _ = crate::utils::copy::copy_to_clipboard(&content);
                }
                Command::none()
            }


        Message::ClipboardDelete => {
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
            Command::none()
        }

        Message::PrevWallpaper => {
            if let Some(index) = &launcher.wallpaper_index {
                if launcher.wallpaper_selected_index > 0 {
                    launcher.wallpaper_selected_index -= 1;
                } else {
                    launcher.wallpaper_selected_index = index.wallpapers.len() - 1;
                }
                let new_index = launcher.wallpaper_selected_index;
                return Command::perform(async move { new_index }, Message::SetWallpaper);
            }
            Command::none()
        }

        Message::NextWallpaper => {
            if let Some(index) = &launcher.wallpaper_index {
                if launcher.wallpaper_selected_index < index.wallpapers.len() - 1 {
                    launcher.wallpaper_selected_index += 1;
                } else {
                    launcher.wallpaper_selected_index = 0;
                }
                let new_index = launcher.wallpaper_selected_index;
                return Command::perform(async move { new_index }, Message::SetWallpaper);
            }
            Command::none()
        }
        Message::SetWallpaper(idx) => {
            if let Some(index) = &launcher.wallpaper_index {
                if let Some(entry) = index.wallpapers.get(idx) {
                    let manager = crate::utils::wallpaper_manager::WallpaperManager::new(index.wallpaper_dir.clone());
                    manager.set_wallpaper(entry);
                    manager.set_last_wallpaper(&entry.path);
                }
            }
            Command::none()
        }

        Message::NoOp => Command::none(),
        
        Message::FocusSearchBar => {
            // Focus the search bar input
            iced::widget::operation::focus(launcher.search_bar.input_id.clone())
        }
        
        Message::WindowReady => {
            // Handled by main.rs
            Command::none()
        }
        
        Message::ShowWindow => {
            // Window show is handled by main.rs
            eprintln!("[IPC] ShowWindow message received");
            Command::none()
        }
        
        Message::HideWindow => {
            // Window hide is handled by main.rs
            eprintln!("[IPC] HideWindow message received");
            Command::none()
        }
        
        Message::NewLayerShell { .. } => {
            // Handled by layershell
            Command::none()
        }
        
        Message::Close(_id) => {
            // Handled by main.rs
            Command::none()
        }
        
        Message::WindowClosed(_id) => {
            // Handled by main.rs
            Command::none()
        }
        
        Message::AppLaunched => {
            // Handled by main.rs - close the window
            Command::none()
        }
        
        // Handle all layershell auto-generated messages (multi-window variants)
        Message::AnchorChange { .. } => Command::none(),
        Message::LayerChange { .. } => Command::none(),
        Message::AnchorSizeChange { .. } => Command::none(),
        Message::MarginChange { .. } => Command::none(),
        Message::SizeChange { .. } => Command::none(),
        Message::ExclusiveZoneChange { .. } => Command::none(),
        Message::SetInputRegion { .. } => Command::none(),
        Message::VirtualKeyboardPressed { .. } => Command::none(),
        Message::NewBaseWindow { .. } => Command::none(),
        Message::NewPopUp { .. } => Command::none(),
        Message::NewMenu { .. } => Command::none(),
        Message::NewInputPanel { .. } => Command::none(),
        Message::RemoveWindow(_) => Command::none(),
        Message::ForgetLastOutput => Command::none(),
    }
}