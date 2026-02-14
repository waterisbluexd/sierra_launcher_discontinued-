use iced::Color;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use crate::config::{Config, ThemeConfig};

// Global theme cache to avoid repeated file reads
static THEME_CACHE: OnceLock<Arc<Mutex<Option<Theme>>>> = OnceLock::new();

/// Get the global theme cache
fn get_theme_cache() -> &'static Arc<Mutex<Option<Theme>>> {
    THEME_CACHE.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Pre-load theme into cache (called on daemon startup)
pub fn preload_theme(config: &Config) {
    let theme = Theme::load_from_config_uncached(config);
    let cache = get_theme_cache();
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(theme);
        eprintln!("[Theme] ✓ Theme pre-loaded into cache");
    }
}

/// Get cached theme or load if not cached
pub fn get_cached_theme(config: &Config) -> Theme {
    let cache = get_theme_cache();
    if let Ok(guard) = cache.lock() {
        if let Some(ref theme) = *guard {
            return theme.clone();
        }
    }
    // Not cached, load and cache
    let theme = Theme::load_from_config_uncached(config);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(theme.clone());
    }
    theme
}

/// Clear theme cache (when pywal colors change)
pub fn clear_theme_cache() {
    let cache = get_theme_cache();
    if let Ok(mut guard) = cache.lock() {
        *guard = None;
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalColors {
    pub special: SpecialColors,
    pub colors: PaletteColors,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpecialColors {
    pub background: String,
    pub foreground: String,
    #[allow(dead_code)]
    pub cursor: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteColors {
    pub color0: String,
    pub color1: String,
    pub color2: String,
    pub color3: String,
    pub color4: String,
    pub color5: String,
    pub color6: String,
    pub color7: String,
    pub color8: String,
    pub color9: String,
    pub color10: String,
    pub color11: String,
    pub color12: String,
    pub color13: String,
    pub color14: String,
    pub color15: String,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    #[allow(dead_code)]
    pub accent: Color,
    #[allow(dead_code)]
    pub color0: Color,
    #[allow(dead_code)]
    pub color1: Color,
    #[allow(dead_code)]
    pub color2: Color,
    #[allow(dead_code)]
    pub color3: Color,
    #[allow(dead_code)]
    pub color4: Color,
    #[allow(dead_code)]
    pub color5: Color,
    #[allow(dead_code)]
    pub color6: Color,
    #[allow(dead_code)]
    pub color7: Color,
    #[allow(dead_code)]
    pub color8: Color,
    #[allow(dead_code)]
    pub color9: Color,
    #[allow(dead_code)]
    pub color10: Color,
    #[allow(dead_code)]
    pub color11: Color,
    #[allow(dead_code)]
    pub color12: Color,
    #[allow(dead_code)]
    pub color13: Color,
    #[allow(dead_code)]
    pub color14: Color,
    #[allow(dead_code)]
    pub color15: Color,
}

impl WalColors {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")?;
        let path = PathBuf::from(home).join(".cache/wal/colors.json");
        let contents = fs::read_to_string(path)?;
        let colors: WalColors = serde_json::from_str(&contents)?;
        Ok(colors)
    }

    pub fn to_theme(&self) -> Theme {
        Theme {
            background: hex_to_color(&self.special.background),
            foreground: hex_to_color(&self.special.foreground),
            border: hex_to_color(&self.colors.color7),
            accent: hex_to_color(&self.colors.color4),
            color0: hex_to_color(&self.colors.color0),
            color1: hex_to_color(&self.colors.color1),
            color2: hex_to_color(&self.colors.color2),
            color3: hex_to_color(&self.colors.color3),
            color4: hex_to_color(&self.colors.color4),
            color5: hex_to_color(&self.colors.color5),
            color6: hex_to_color(&self.colors.color6),
            color7: hex_to_color(&self.colors.color7),
            color8: hex_to_color(&self.colors.color8),
            color9: hex_to_color(&self.colors.color9),
            color10: hex_to_color(&self.colors.color10),
            color11: hex_to_color(&self.colors.color11),
            color12: hex_to_color(&self.colors.color12),
            color13: hex_to_color(&self.colors.color13),
            color14: hex_to_color(&self.colors.color14),
            color15: hex_to_color(&self.colors.color15),
        }
    }
}

impl Theme {
    /// Load theme based on config preferences (uses cache if available)
    /// Priority: pywal (if enabled) > custom theme > default
    pub fn load_from_config(config: &Config) -> Self {
        get_cached_theme(config)
    }
    
    /// Load theme without using cache (for pre-loading)
    fn load_from_config_uncached(config: &Config) -> Self {
        // If pywal is enabled, try to load it FIRST
        if config.use_pywal {
            if let Ok(wal_colors) = WalColors::load() {
                eprintln!("[Theme] Using pywal theme");
                return wal_colors.to_theme();
            } else {
                eprintln!("[Theme] Pywal enabled but colors.json not found");
            }
        }
        
        // Use custom theme from config if available
        if let Some(ref theme_config) = config.custom_theme {
            eprintln!("[Theme] Using custom theme from config");
            return Self::from_config_theme(theme_config);
        }
        
        // Fallback to default theme
        eprintln!("[Theme] Using default theme");
        Self::default()
    }
    
    /// Create theme from config theme
    fn from_config_theme(theme_config: &ThemeConfig) -> Self {
        Theme {
            background: theme_config.background
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgba(0.15, 0.15, 0.18, 0.82)),
            foreground: theme_config.foreground
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::WHITE),
            border: theme_config.border
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.5, 0.5, 0.5)),
            accent: theme_config.accent
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.6, 0.6, 0.6)),
            color0: theme_config.color0
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::BLACK),
            color1: theme_config.color1
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.8, 0.0, 0.0)),
            color2: theme_config.color2
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.0, 0.8, 0.0)),
            color3: theme_config.color3
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.8, 0.8, 0.0)),
            color4: theme_config.color4
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.0, 0.0, 0.8)),
            color5: theme_config.color5
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.8, 0.0, 0.8)),
            color6: theme_config.color6
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.0, 0.8, 0.8)),
            color7: theme_config.color7
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.7, 0.7, 0.7)),
            color8: theme_config.color8
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.5, 0.5, 0.5)),
            color9: theme_config.color9
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(1.0, 0.0, 0.0)),
            color10: theme_config.color10
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.0, 1.0, 0.0)),
            color11: theme_config.color11
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(1.0, 1.0, 0.0)),
            color12: theme_config.color12
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.0, 0.0, 1.0)),
            color13: theme_config.color13
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(1.0, 0.0, 1.0)),
            color14: theme_config.color14
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::from_rgb(0.0, 1.0, 1.0)),
            color15: theme_config.color15
                .as_ref()
                .map(|s| Config::hex_to_color(s))
                .unwrap_or(Color::WHITE),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: Color::from_rgba(0.15, 0.15, 0.18, 0.82),
            foreground: Color::WHITE,
            border: Color::from_rgb(0.5, 0.5, 0.5),
            accent: Color::from_rgb(0.6, 0.6, 0.6),
            color0: Color::BLACK,
            color1: Color::from_rgb(0.8, 0.0, 0.0),
            color2: Color::from_rgb(0.0, 0.8, 0.0),
            color3: Color::from_rgb(0.8, 0.8, 0.0),
            color4: Color::from_rgb(0.0, 0.0, 0.8),
            color5: Color::from_rgb(0.8, 0.0, 0.8),
            color6: Color::from_rgb(0.0, 0.8, 0.8),
            color7: Color::from_rgb(0.7, 0.7, 0.7),
            color8: Color::from_rgb(0.5, 0.5, 0.5),
            color9: Color::from_rgb(1.0, 0.0, 0.0),
            color10: Color::from_rgb(0.0, 1.0, 0.0),
            color11: Color::from_rgb(1.0, 1.0, 0.0),
            color12: Color::from_rgb(0.0, 0.0, 1.0),
            color13: Color::from_rgb(1.0, 0.0, 1.0),
            color14: Color::from_rgb(0.0, 1.0, 1.0),
            color15: Color::WHITE,
        }
    }
}

fn hex_to_color(hex: &str) -> Color {
    Config::hex_to_color(hex)
}