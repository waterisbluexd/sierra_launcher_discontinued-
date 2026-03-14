use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use iced::{Font, Color};
use crate::Anchor;
use crate::app::message::{WINDOW_HEIGHT, POPUP_GAP};

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigFile {
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub use_pywal: Option<bool>,
    pub theme: Option<ThemeConfig>,
    pub title_text: Option<String>,
    pub title_animation: Option<String>,
    pub title_animation_speed: Option<f32>,
    pub wallpaper_dir: Option<String>,
    pub weather_location: Option<String>,
    pub location: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ThemeConfig {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub border: Option<String>,
    pub accent: Option<String>,
    pub color0: Option<String>,
    pub color1: Option<String>,
    pub color2: Option<String>,
    pub color3: Option<String>,
    pub color4: Option<String>,
    pub color5: Option<String>,
    pub color6: Option<String>,
    pub color7: Option<String>,
    pub color8: Option<String>,
    pub color9: Option<String>,
    pub color10: Option<String>,
    pub color11: Option<String>,
    pub color12: Option<String>,
    pub color13: Option<String>,
    pub color14: Option<String>,
    pub color15: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub font_name: Option<String>,
    pub font_size: Option<f32>,
    pub use_pywal: bool,
    pub custom_theme: Option<ThemeConfig>,
    pub title_text: String,
    pub title_animation: String,
    pub title_animation_speed: f32,
    pub wallpaper_dir: Option<PathBuf>,
    pub weather_location: Option<String>,
    pub location: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();

        let config_file: ConfigFile = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|s| toml::from_str(&s).ok())
                .unwrap_or_else(Self::default_config_file)
        } else {
            Self::default_config_file()
        };

        let wallpaper_dir = config_file
            .wallpaper_dir
            .and_then(Self::expand_path)
            .filter(|p| p.exists());

        Self {
            font_name: config_file.font,
            font_size: config_file.font_size,
            use_pywal: config_file.use_pywal.unwrap_or(false),
            custom_theme: config_file.theme,
            title_text: config_file
                .title_text
                .unwrap_or_else(|| " sierra-launcher ".to_string()),
            title_animation: config_file
                .title_animation
                .unwrap_or_else(|| "Wave".to_string()),
            title_animation_speed: config_file
                .title_animation_speed
                .unwrap_or(80.0),
            wallpaper_dir,
            weather_location: config_file.weather_location,
            location: config_file
                .location
                .unwrap_or_else(|| "bottom".to_string()),
            x: config_file.x,
            y: config_file.y,
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sierra")
            .join("Sierra")
    }

    fn default_config_file() -> ConfigFile {
        ConfigFile {
            font: Some("Monospace".to_string()),
            font_size: Some(14.0),
            use_pywal: Some(false),
            theme: None,
            title_text: Some(" sierra-launcher ".to_string()),
            title_animation: Some("Wave".to_string()),
            title_animation_speed: Some(80.0),
            wallpaper_dir: Some("~/Pictures/Wallpapers".to_string()),
            weather_location: None,
            location: Some("bottom".to_string()),
            x: None,
            y: None,
        }
    }

    fn expand_path(input: String) -> Option<PathBuf> {
        if input.starts_with("~/") {
            dirs::home_dir().map(|h| h.join(&input[2..]))
        } else {
            Some(PathBuf::from(input))
        }
    }

    pub fn get_font(&self) -> Font {
        self.font_name
            .as_ref()
            .map(|name| {
                let static_name: &'static str =
                    Box::leak(name.clone().into_boxed_str());
                Font::with_name(static_name)
            })
            .unwrap_or(Font::default())
    }

    pub fn get_animation_mode(&self) -> crate::panels::title_color::AnimationMode {
        use crate::panels::title_color::AnimationMode;
        match self.title_animation.as_str() {
            "Rainbow" => AnimationMode::Rainbow,
            "Wave" => AnimationMode::Wave,
            "InOutWave" => AnimationMode::InOutWave,
            "Pulse" => AnimationMode::Pulse,
            "Sparkle" => AnimationMode::Sparkle,
            "Gradient" => AnimationMode::Gradient,
            _ => AnimationMode::Wave,
        }
    }

    pub fn get_anchor(&self) -> Anchor {
        match self.location.to_lowercase().as_str() {
            "top" => Anchor::Top,
            "bottom" => Anchor::Bottom,
            "left" => Anchor::Left,
            "right" => Anchor::Right,
            "center" => Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            "top-left" => Anchor::Top | Anchor::Left,
            "top-right" => Anchor::Top | Anchor::Right,
            "bottom-left" => Anchor::Bottom | Anchor::Left,
            "bottom-right" => Anchor::Bottom | Anchor::Right,
            _ => Anchor::Bottom,
        }
    }

    /// Returns margin as (left, right, top, bottom)
    /// x corresponds to left/right margin, y corresponds to top/bottom margin
    pub fn get_margin(&self) -> (i32, i32, i32, i32) {
        let location = self.location.to_lowercase();
        let x = self.x.unwrap_or(0);
        let y = self.y.unwrap_or(4); // default bottom margin
        
        match location.as_str() {
            "top" => (0, 0, y, 0),
            "bottom" => (x, 0, 0, y),
            "left" => (x, 0, y, 0),
            "right" => (0, x, y, 0),
            "center" => (x, x, y, y),
            "top-left" => (x, 0, y, 0),
            "top-right" => (0, x, y, 0),
            "bottom-left" => (x, 0, 0, y),
            "bottom-right" => (0, x, 0, y),
            _ => (x, 0, 0, y), // default bottom-left
        }
    }

    /// Returns popup margin (positioned above main window)
    /// Format: (left, right, top, bottom)
    pub fn get_popup_margin(&self) -> (i32, i32, i32, i32) {
        let x = self.x.unwrap_or(0);
        // Popup is always positioned above main window
        // Main window has height WINDOW_HEIGHT, so popup needs margin to be above it
        (x, 0, (WINDOW_HEIGHT + 4 + POPUP_GAP) as i32, 0)
    }

    pub fn hex_to_color(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::from_rgb(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                );
            }
        }
        Color::WHITE
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_name: Some("Monospace".to_string()),
            font_size: Some(14.0),
            use_pywal: false,
            custom_theme: None,
            title_text: " sierra-launcher ".to_string(),
            title_animation: "Wave".to_string(),
            title_animation_speed: 80.0,
            wallpaper_dir: None,
            weather_location: None,
            location: "bottom".to_string(),
            x: None,
            y: None,
        }
    }
}
