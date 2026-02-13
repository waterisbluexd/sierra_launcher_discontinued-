use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use iced::{Font, Color};

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigFile {
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub use_pywal: Option<bool>,
    pub theme: Option<ThemeConfig>,
    pub title_text: Option<String>,
    pub title_animation: Option<String>,
    pub wallpaper_dir: Option<String>,
    pub weather_location: Option<String>,
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
    pub wallpaper_dir: Option<PathBuf>,
    pub weather_location: Option<String>,
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
            wallpaper_dir,
            weather_location: config_file.weather_location,
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
            wallpaper_dir: Some("~/Pictures/Wallpapers".to_string()),
            weather_location: None,
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
            wallpaper_dir: None,
            weather_location: None,
        }
    }
}
