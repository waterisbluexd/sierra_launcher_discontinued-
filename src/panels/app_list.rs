use iced::widget::{column, container, row, scrollable, text};
use iced::{Border, Element, Length, Task};
use gio::prelude::*;
use gio::{AppLaunchContext, DesktopAppInfo};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use crate::utils::theme::Theme;

#[derive(Debug, Clone)]
pub enum Message {
    SearchInput(String),
    ArrowUp,
    ArrowDown,
    LaunchSelected,
}

#[derive(Debug, Clone)]
pub struct App {
    id: String,
    name: String,
    name_lower: String,
    description_lower: Option<String>,
}

static APP_CACHE: OnceLock<Vec<App>> = OnceLock::new();

// Track loading state
static LOADING_STATE: OnceLock<Arc<Mutex<LoadingState>>> = OnceLock::new();

/// Pre-warm the app cache on daemon startup for instant first show
pub fn prewarm_cache() {
    let state = LOADING_STATE.get_or_init(|| Arc::new(Mutex::new(LoadingState::NotStarted)));
    
    let mut state_lock = state.lock().unwrap();
    if *state_lock != LoadingState::NotStarted {
        return;
    }
    
    *state_lock = LoadingState::Loading;
    drop(state_lock);
    
    // Load apps synchronously (this is called in background thread)
    APP_CACHE.get_or_init(|| load_desktop_apps_impl());
    
    let state = LOADING_STATE.get().unwrap();
    *state.lock().unwrap() = LoadingState::Loaded;
}

/// Internal function to load desktop apps (used by prewarm_cache)
fn load_desktop_apps_impl() -> Vec<App> {
    eprintln!("[AppList] Loading desktop applications...");
    
    let mut apps: Vec<App> = gio::AppInfo::all()
        .into_iter()
        .filter_map(|app| {
            let desktop = app.downcast::<DesktopAppInfo>().ok()?;

            if !desktop.should_show() {
                return None;
            }

            let name = desktop.name().to_string();
            let name_lower = name.to_lowercase();
            let description_lower = desktop.description().map(|d| d.to_lowercase());

            Some(App {
                id: desktop.id()?.to_string(),
                name,
                name_lower,
                description_lower,
            })
        })
        .collect();

    // Sort apps alphabetically
    apps.sort_unstable_by(|a, b| a.name_lower.cmp(&b.name_lower));
    
    eprintln!("[AppList] ✓ Loaded and sorted {} desktop apps", apps.len());
    apps
}

#[derive(Debug, Clone, PartialEq)]
enum LoadingState {
    NotStarted,
    Loading,
    Loaded,
}

pub struct AppList {
    filtered_indices: Vec<usize>,
    pub search_query: String,
    pub selected_index: usize,
    scroll_id: iced::widget::Id,
    window_size: usize,
    window_start: usize,
    is_loading: bool,
}

impl AppList {
    pub fn new() -> Self {
        // Initialize loading state tracker
        LOADING_STATE.get_or_init(|| Arc::new(Mutex::new(LoadingState::NotStarted)));
        
        Self {
            filtered_indices: Vec::new(), // Start empty
            search_query: String::new(),
            selected_index: 0,
            scroll_id: iced::widget::Id::unique(),
            window_size: 17,
            window_start: 0,
            is_loading: false, // Not loading yet - wait for start_loading() call
        }
    }

    /// Trigger lazy loading of apps in background thread
    /// Call this AFTER the first frame is rendered
    pub fn start_loading(&mut self) {
        let state = LOADING_STATE.get().unwrap().clone();
        let mut state_lock = state.lock().unwrap();
        
        // Only start loading once
        if *state_lock != LoadingState::NotStarted {
            drop(state_lock);
            
            // If already loaded, populate immediately
            if APP_CACHE.get().is_some() {
                let total_apps = Self::all_apps().len();
                self.filtered_indices = (0..total_apps).collect();
                self.is_loading = false;
                eprintln!("[AppList] Apps already cached - {} total", total_apps);
            }
            return;
        }
        
        *state_lock = LoadingState::Loading;
        drop(state_lock);
        
        self.is_loading = true;
        eprintln!("[AppList] Starting lazy app loading...");
        
        // Load apps in background thread
        thread::spawn(move || {
            let start = std::time::Instant::now();
            
            // This will initialize APP_CACHE
            APP_CACHE.get_or_init(|| load_desktop_apps_impl());
            
            // Update state
            let state = LOADING_STATE.get().unwrap();
            *state.lock().unwrap() = LoadingState::Loaded;
            
            eprintln!("[AppList] ✓ Loaded {} apps in {:?}", 
                APP_CACHE.get().unwrap().len(), 
                start.elapsed()
            );
        });
    }

    /// Check if apps are loaded and update filtered list
    /// Returns true if apps just finished loading
    pub fn check_loaded(&mut self) -> bool {
        if !self.is_loading {
            return false;
        }
        
        let state = LOADING_STATE.get().unwrap();
        let state_lock = state.lock().unwrap();
        
        if *state_lock == LoadingState::Loaded {
            drop(state_lock);
            
            // Apps are now available - populate the list
            let total_apps = Self::all_apps().len();
            self.filtered_indices = (0..total_apps).collect();
            self.is_loading = false;
            
            eprintln!("[AppList] ✓ Apps populated - {} total", total_apps);
            return true;
        }
        
        false
    }

    fn all_apps() -> &'static [App] {
        APP_CACHE.get().map_or(&[], |v| v.as_slice())
    }

    fn filter_apps(&mut self) {
        let apps = Self::all_apps();
        
        if apps.is_empty() {
            self.filtered_indices.clear();
            return;
        }
        
        if self.search_query.is_empty() {
            self.filtered_indices = (0..apps.len()).collect();
        } else {
            let q = self.search_query.to_lowercase();

            self.filtered_indices.clear();
            self.filtered_indices.extend(
                apps.iter()
                    .enumerate()
                    .filter(|(_, app)| {
                        app.name_lower.contains(&q)
                            || app
                                .description_lower
                                .as_deref()
                                .map_or(false, |d| d.contains(&q))
                    })
                    .map(|(idx, _)| idx),
            );
        }

        if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = 0;
        }

        self.update_window();
    }

    fn update_window(&mut self) {
        if self.filtered_indices.is_empty() {
            self.window_start = 0;
            return;
        }

        if self.selected_index >= self.window_start + self.window_size {
            self.window_start = self.selected_index + 1 - self.window_size;
        } else if self.selected_index < self.window_start {
            self.window_start = self.selected_index;
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchInput(query) => {
                self.search_query = query;
                self.filter_apps();
                self.window_start = 0;
                Task::none()
            }
            Message::ArrowUp => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_window();
                }
                Task::none()
            }
            Message::ArrowDown => {
                if !self.filtered_indices.is_empty()
                    && self.selected_index < self.filtered_indices.len() - 1
                {
                    self.selected_index += 1;
                    self.update_window();
                }
                Task::none()
            }
            Message::LaunchSelected => {
                self.launch_selected();
                Task::none()
            }
        }
    }

    fn launch_selected(&self) {
        if let Some(&app_idx) = self.filtered_indices.get(self.selected_index) {
            let app = &Self::all_apps()[app_idx];
            if let Some(info) = DesktopAppInfo::new(&app.id) {
                let _ = info.launch(&[], AppLaunchContext::NONE);
            }
        }
    }

    pub fn view<'a>(
        &'a self,
        theme: &'a Theme,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let mut items = column![].spacing(1);
        
        // Show loading message if apps aren't loaded yet
        if self.is_loading || Self::all_apps().is_empty() {
            items = items.push(
                container(
                    text("Loading applications...")
                        .font(font)
                        .size(font_size)
                        .color(theme.color6)
                )
                .padding(20)
                .width(Length::Fill)
                .center_x(Length::Fill)
            );
        } else if self.filtered_indices.is_empty() {
            // Show "no results" if search yields nothing
            items = items.push(
                container(
                    text(if self.search_query.is_empty() {
                        "No applications found"
                    } else {
                        "No matching applications"
                    })
                    .font(font)
                    .size(font_size)
                    .color(theme.color6)
                )
                .padding(20)
                .width(Length::Fill)
                .center_x(Length::Fill)
            );
        } else {
            // Normal app list rendering
            let window_end = (self.window_start + self.window_size).min(self.filtered_indices.len());
            let apps = Self::all_apps();

            for idx in self.window_start..window_end {
                let &app_idx = &self.filtered_indices[idx];
                let app = &apps[app_idx];
                let selected = idx == self.selected_index;

                let bg = if selected {
                    Some(theme.color3.into())
                } else {
                    None
                };

                let fg = if selected {
                    theme.background
                } else {
                    theme.foreground
                };

                let content = if selected {
                    row![
                        text(">>").font(font).size(font_size).color(fg),
                        text(&app.name).font(font).size(font_size).color(fg),
                    ]
                    .spacing(4)
                } else {
                    row![
                        text("  ").font(font).size(font_size).color(fg),
                        text(&app.name).font(font).size(font_size).color(fg),
                    ]
                    .spacing(4)
                };

                items = items.push(
                    container(content)
                        .padding([2, 4])
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: bg,
                            border: Border::default(),
                            ..Default::default()
                        }),
                );
            }
        }

        scrollable(items)
            .id(self.scroll_id.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_, _| scrollable::Style {
                container: container::Style::default(),
                vertical_rail: scrollable::Rail {
                    background: None,
                    border: Border::default(),
                    scroller: scrollable::Scroller {
                        background: iced::Background::Color(iced::Color::TRANSPARENT),
                        border: Border::default(),
                    },
                },
                horizontal_rail: scrollable::Rail {
                    background: None,
                    border: Border::default(),
                    scroller: scrollable::Scroller {
                        background: iced::Background::Color(iced::Color::TRANSPARENT),
                        border: Border::default(),
                    },
                },
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: iced::Background::Color(iced::Color::TRANSPARENT),
                    border: Border::default(),
                    icon: iced::Color::TRANSPARENT,
                    shadow: iced::Shadow::default(),
                },
            })
            .into()
    }
}