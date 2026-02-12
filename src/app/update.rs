use iced::Task as Command;
use iced::Event;
use iced::widget::operation::focus;
use iced::keyboard;
use keyboard::key::Named;
use iced::mouse;

use crate::app::state::{Launcher, Panel, Direction};
use crate::app::message::Message;
use crate::panels::{search_bar, app_list};
use crate::utils::theme::WalColors;
use crate::utils::watcher::ColorWatcher; 
use std::time::{Duration, Instant};

pub fn update(launcher: &mut Launcher, message: Message) -> Command<Message> {
    match message {
        Message::IcedEvent(event) => {
            match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    match key {
                        keyboard::Key::Named(Named::Escape) => {
                            std::process::exit(0);
                        }
                            if launcher.clipboard_visible {
                                return Command::perform(async {}, |_| Message::ClipboardSelect);
                            } else {
                                // Launch selected app
                                let _ = launcher.app_list.update(app_list::Message::LaunchSelected);
                                std::process::exit(0);
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
                                // FIX: Toggle clipboard visibility
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            } else {
                                return Command::perform(async {}, |_| Message::CyclePanel(Direction::Left));
                            }
                        }
                        
                        keyboard::Key::Named(Named::ArrowRight) => {
                            if modifiers.shift() {
                                // FIX: Toggle clipboard visibility
                                launcher.clipboard_visible = !launcher.clipboard_visible;
                            } else {
                                return Command::perform(async {}, |_| Message::CyclePanel(Direction::Right));
                            }
                        }

                        keyboard::Key::Named(Named::Backspace) => {
                            if !launcher.clipboard_visible && !launcher.search_bar.input_value.is_empty() {
                                // Handle backspace for search input
                                launcher.search_bar.input_value.pop();
                                let _ = launcher.app_list.update(app_list::Message::SearchInput(launcher.search_bar.input_value.clone()));
                            }
                        }

                        keyboard::Key::Character(c) => {
                            if launcher.clipboard_visible && modifiers.control() && c.as_str() == "d" {
                                return Command::perform(async {}, |_| Message::ClipboardDelete);
                            } else if !launcher.clipboard_visible && !modifiers.control() && !modifiers.alt() && !modifiers.logo() {
                                // Type into search bar even when not focused
                                launcher.search_bar.input_value.push_str(c.as_str());
                                let _ = launcher.app_list.update(app_list::Message::SearchInput(launcher.search_bar.input_value.clone()));
                            }
                        }
                        
                        _ => {}
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
                
                // Trigger lazy loading of apps in background
                launcher.app_list.start_loading();
                eprintln!("[Main] Triggered lazy app loading");
                launcher.system_panel.start();
                
                // FIX: Load wallpaper index asynchronously on first frame
                if launcher.wallpaper_index.is_none() {
                    if let Some(ref wallpaper_dir) = launcher.config.wallpaper_dir {
                        let wp_dir = wallpaper_dir.clone();
                        std::thread::spawn(move || {
                            let manager = crate::utils::wallpaper_manager::WallpaperManager::new(wp_dir);
                            if let Some(_index) = manager.load_index() {
                                eprintln!("[Wallpaper] Index loaded in background");
                            }
                        });
                    }
                }
                
                if launcher.config.use_pywal && launcher.watcher.is_none() {
                    launcher.watcher = ColorWatcher::new().ok();
                }
                
                return focus(launcher.search_bar.input_id.clone());
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
                        // Debounce: skip if we reloaded less than 1 second ago
                        if now.duration_since(launcher.last_pywal_reload) > Duration::from_secs(1) {
                            if watcher.check_for_changes() {
                                if let Ok(wal_colors) = WalColors::load() {
                                    launcher.theme = wal_colors.to_theme();
                                    launcher.last_pywal_reload = now;
                                    eprintln!("[Pywal] Theme reloaded");
                                }
                            }
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
                    std::process::exit(0);
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
            
            // FIX: Load wallpaper index when switching to wallpaper panel
            if launcher.current_panel == Panel::Wallpaper && launcher.wallpaper_index.is_none() {
                if let Some(ref wallpaper_dir) = launcher.config.wallpaper_dir {
                    let manager = crate::utils::wallpaper_manager::WallpaperManager::new(wallpaper_dir.clone());
                    launcher.wallpaper_index = manager.load_index();
                    
                    // Restore selected index
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
            
            std::thread::spawn(|| {
                let _ = std::process::Command::new("systemctl")
                    .arg("poweroff")
                    .output();
            });
            Command::none()
        }

        Message::RestartTheSystem => {
            launcher.control_center_visible = false;
            
            std::thread::spawn(|| {
                let _ = std::process::Command::new("systemctl")
                    .arg("reboot")
                    .output();
            });
            Command::none()
        }

        Message::SleepModeTheSystem => {
            launcher.control_center_visible = false;
            
            let _ = std::process::Command::new("bash")
                .arg("-c")
                .arg("(sleep 0.5 && systemctl suspend) &")
                .spawn();
            
            std::process::exit(0);
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
        
        Message::ShowWindow => {
            // Reset search bar and refocus
            launcher.search_bar.input_value.clear();
            let _ = launcher.app_list.update(app_list::Message::SearchInput(String::new()));
            launcher.clipboard_selected_index = 0;
            return focus(launcher.search_bar.input_id.clone());
        }
    }
}