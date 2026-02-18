use iced::widget::{container, text, column, row, stack};
use iced::{Element, Border, Color, Length, Padding};
use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

const CACHE_FILE: &str = ".cache/sierra/weather.cache";
const CACHE_VALIDITY_SECS: u64 = 1800;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    location: String,
    temp: String,
    feels_like: String,
    condition: String,
    humidity: String,
    wind_speed: String,
    wind_dir: String,
    hourly: Vec<HourlyData>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyData {
    time: String,
    #[serde(rename = "tempC")]
    temp_c: String,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "precipMM")]
    precip_mm: String,
}

#[derive(Deserialize)]
struct WttrResponse {
    current_condition: Vec<CurrentCondition>,
    weather: Vec<WeatherDay>,
}

#[derive(Deserialize)]
struct CurrentCondition {
    #[serde(rename = "temp_C")]
    temp_c: String,
    #[serde(rename = "FeelsLikeC")]
    feels_like_c: String,
    #[serde(rename = "weatherDesc")]
    weather_desc: Vec<WeatherDesc>,
    humidity: String,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "winddir16Point")]
    winddir16_point: String,
}

#[derive(Deserialize)]
struct WeatherDesc {
    value: String,
}

#[derive(Deserialize)]
struct WeatherDay {
    hourly: Vec<HourlyForecast>,
}

#[derive(Deserialize)]
struct HourlyForecast {
    time: String,
    #[serde(rename = "tempC")]
    temp_c: String,
    #[serde(rename = "windspeedKmph")]
    windspeed_kmph: String,
    #[serde(rename = "precipMM")]
    precip_mm: String,
}

pub struct WeatherPanel {
    weather_data: Arc<Mutex<Option<WeatherData>>>,
    is_updating: Arc<Mutex<bool>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl WeatherPanel {
    pub fn new() -> Self {
        Self::with_location(None)
    }
    
    pub fn with_location(location: Option<String>) -> Self {
        let weather_data = Arc::new(Mutex::new(None));
        let is_updating = Arc::new(Mutex::new(false));
        let last_error = Arc::new(Mutex::new(None));
        
        let location_key = location.clone().unwrap_or_else(|| "auto".to_string());
        let location_normalized = location_key.split(',').next().unwrap_or(&location_key).trim().to_lowercase();
        
        if let Some(cached) = Self::load_from_cache() {
            let cached_location_normalized = cached.location.split(',').next().unwrap_or(&cached.location).trim().to_lowercase();
            let location_matches = cached_location_normalized == location_normalized;
            
            if location_matches {
                eprintln!("[Weather] ✓ Loaded cached weather data for: {}", cached.location);
                *weather_data.lock().unwrap_or_else(|e| e.into_inner()) = Some(cached.clone());
                
                if let Ok(age) = SystemTime::now().duration_since(cached.cached_at) {
                    if age.as_secs() < CACHE_VALIDITY_SECS {
                        eprintln!("[Weather] ✓ Cache is fresh ({} sec old), skipping fetch", age.as_secs());
                        return Self { weather_data, is_updating, last_error };
                    } else {
                        eprintln!("[Weather] Cache expired ({} sec old), fetching fresh data...", age.as_secs());
                    }
                }
            } else {
                eprintln!("[Weather] Cache location mismatch (cached: '{}', requested: '{}'), fetching fresh data...", cached.location, location_key);
            }
        } else {
            eprintln!("[Weather] No cache found, fetching weather data...");
        }
        
        let weather_clone = Arc::clone(&weather_data);
        let updating_clone = Arc::clone(&is_updating);
        let error_clone = Arc::clone(&last_error);
        
        *is_updating.lock().unwrap_or_else(|e| e.into_inner()) = true;
        
        thread::spawn(move || {
            let result = std::panic::catch_unwind(|| {
                match Self::fetch_weather_data(&location) {
                    Ok(new_data) => {
                        eprintln!("[Weather] ✓ Fetched fresh weather data for: {}", new_data.location);
                        *weather_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(new_data.clone());
                        *error_clone.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        
                        if let Err(e) = Self::save_to_cache(&new_data) {
                            eprintln!("[Weather] ⚠ Failed to save cache: {}", e);
                        } else {
                            eprintln!("[Weather] ✓ Saved to cache");
                        }
                    }
                    Err(e) => {
                        eprintln!("[Weather] ⚠ Failed to fetch weather: {}", e);
                        *error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some(e.to_string());
                    }
                }
            });
            
            if result.is_err() {
                eprintln!("[Weather] ⚠ Weather fetch panicked");
                *error_clone.lock().unwrap_or_else(|e| e.into_inner()) = Some("Weather fetch crashed".to_string());
            }
            
            *updating_clone.lock().unwrap_or_else(|e| e.into_inner()) = false;
        });

        Self { weather_data, is_updating, last_error }
    }

    fn get_cache_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CACHE_FILE)
    }

    fn load_from_cache() -> Option<WeatherData> {
        let path = Self::get_cache_path();
        let content = fs::read(&path).ok()?;
        bincode::deserialize(&content).ok()
    }

    fn save_to_cache(data: &WeatherData) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_cache_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = bincode::serialize(data)?;
        fs::write(&path, encoded)?;
        Ok(())
    }

    #[inline]
    fn get_greeting() -> &'static str {
        let hour = Local::now().hour();
        match hour {
            5..=11 => "Good Morning",
            12..=16 => "Good Afternoon",
            17..=20 => "Good Evening",
            _ => "Good Night",
        }
    }

    #[inline]
    fn get_weather_art(condition: &str) -> &'static [&'static str] {
        let condition_lower = condition.to_lowercase();
        if condition_lower.contains("sunny") || condition_lower.contains("clear") {
            &["    \\   /    ", "     .-.     ", "  — (   ) —  ", "     `-'     ", "    /   \\    "]
        } else if condition_lower.contains("cloudy") {
            &["             ", "     .--.    ", "  .-(    ).  ", " (___.__)__) ", "             "]
        } else if condition_lower.contains("rain") || condition_lower.contains("shower") {
            &["     .--.    ", "  .-(    ).  ", " (___.__)__) ", "   ‚'‚'‚'‚'   ", "   ‚'‚'‚'‚'   "]
        } else if condition_lower.contains("snow") {
            &["     .--.    ", "  .-(    ).  ", " (___.__)__) ", "   * * * * ", "  * * * * "]
        } else if condition_lower.contains("thunder") || condition_lower.contains("storm") {
            &["     .--.    ", "  .-(    ).  ", " (___.__)__) ", "   ⚡'‚'⚡'‚'   ", "   ‚'⚡'‚'‚'   "]
        } else {
            &["    .--.     ", "   (    )    ", "  (      )   ", " (________)  ", "             "]
        }
    }

    fn fetch_weather_data(location: &Option<String>) -> Result<WeatherData, Box<dyn std::error::Error>> {
        let (url, location_name) = if let Some(loc) = location {
            eprintln!("[Weather] Using configured location: {}", loc);
            let simple_loc = loc.split(',').next().unwrap_or(loc).trim();
            (format!("https://wttr.in/{}?format=j1", simple_loc), simple_loc.to_string())
        } else {
            eprintln!("[Weather] Auto-detecting location via IP...");
            ("https://wttr.in/?format=j1".to_string(), "auto".to_string())
        };
        
        eprintln!("[Weather] Fetching from: {}", url);
        
        let response = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .build()
            .get(&url)
            .set("User-Agent", "curl/7.68.0")
            .call()?;
        
        let weather_resp: WttrResponse = response.into_json()?;

        let current = &weather_resp.current_condition[0];
        
        let mut all_hourly: Vec<HourlyData> = Vec::new();
        for day in &weather_resp.weather {
            for h in &day.hourly {
                all_hourly.push(HourlyData {
                    time: h.time.clone(),
                    temp_c: h.temp_c.clone(),
                    windspeed_kmph: h.windspeed_kmph.clone(),
                    precip_mm: h.precip_mm.clone(),
                });
            }
        }
        
        eprintln!("[Weather] Got {} hourly data points", all_hourly.len());

        Ok(WeatherData {
            location: location_name,
            temp: current.temp_c.clone(),
            feels_like: current.feels_like_c.clone(),
            condition: current.weather_desc[0].value.clone(),
            humidity: current.humidity.clone(),
            wind_speed: current.windspeed_kmph.clone(),
            wind_dir: current.winddir16_point.clone(),
            hourly: all_hourly,
            cached_at: SystemTime::now(),
        })
    }

    fn format_hourly_forecast(hourly: &[HourlyData]) -> Vec<String> {
        const TIME_SLOTS: [&str; 8] = ["0", "300", "600", "900", "1200", "1500", "1800", "2100"];
        const TIME_LABELS: [&str; 8] = ["12am", "3am", "6am", "9am", "12pm", "3pm", "6pm", "9pm"];
        
        let now = Local::now();
        let current_hour = now.hour();
        let current_slot = (current_hour / 3) as usize;
        
        let mut lines = Vec::with_capacity(4);
        
        let mut header = String::with_capacity(50);
        for i in 0..6 {
            let slot_idx = (current_slot + i) % 8;
            header.push_str(&format!("{:>7}", TIME_LABELS[slot_idx]));
        }
        lines.push(header);
        
        let mut temp_line = String::with_capacity(50);
        for i in 0..6 {
            let slot_idx = (current_slot + i) % 8;
            let hour_data = hourly.iter().find(|h| h.time == TIME_SLOTS[slot_idx]);
            if let Some(data) = hour_data {
                temp_line.push_str(&format!("{:>7}", format!("{}°", data.temp_c)));
            } else {
                temp_line.push_str("     --");
            }
        }
        lines.push(temp_line);
        
        let mut wind_line = String::with_capacity(50);
        for i in 0..6 {
            let slot_idx = (current_slot + i) % 8;
            let hour_data = hourly.iter().find(|h| h.time == TIME_SLOTS[slot_idx]);
            if let Some(data) = hour_data {
                wind_line.push_str(&format!("{:>7}", format!("{}k", data.windspeed_kmph)));
            } else {
                wind_line.push_str("    --k");
            }
        }
        lines.push(wind_line);
        
        let mut precip_line = String::with_capacity(50);
        for i in 0..6 {
            let slot_idx = (current_slot + i) % 8;
            let hour_data = hourly.iter().find(|h| h.time == TIME_SLOTS[slot_idx]);
            if let Some(data) = hour_data {
                precip_line.push_str(&format!("{:>7}", format!("{}mm", data.precip_mm)));
            } else {
                precip_line.push_str("   --mm");
            }
        }
        lines.push(precip_line);
        
        lines
    }

    pub fn view<'a, Message: 'a>(
        &self,
        theme: &'a crate::utils::theme::Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let weather_data_guard = self.weather_data.lock().unwrap_or_else(|e| e.into_inner());
        let is_updating = *self.is_updating.lock().unwrap_or_else(|e| e.into_inner());
        let last_error = self.last_error.lock().unwrap_or_else(|e| e.into_inner()).clone();

        let content = if let Some(weather_clone) = weather_data_guard.clone() {
            let greeting = Self::get_greeting();
            let art_lines = Self::get_weather_art(&weather_clone.condition);
            
            let mut art_col = column![].spacing(0);
            for &line in art_lines {
                art_col = art_col.push(
                    text(line)
                        .line_height(1.0)
                        .color(theme.color4)
                        .font(font)
                        .size(font_size)
                );
            }
            
            let mut info_col = column![].spacing(5).padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 0.0 });
            
            info_col = info_col.push(
                text(greeting)
                    .line_height(1.0)
                    .color(theme.color1)
                    .font(font)
                    .size(font_size)
            );
            
            info_col = info_col.push(
                text(weather_clone.condition.clone())
                    .line_height(1.0)
                    .color(theme.foreground)
                    .font(font)
                    .size(font_size)
            );
            
            info_col = info_col.push(
                text(format!("{}°C", weather_clone.temp))
                    .line_height(1.0)
                    .color(theme.color12)
                    .font(font)
                    .size(font_size * 1.2)
            );
            
            info_col = info_col.push(
                row![
                    text("Wind: ")
                        .line_height(1.0)
                        .color(theme.color8)
                        .font(font)
                        .size(font_size),
                    text(format!("{} km/h", weather_clone.wind_speed))
                        .line_height(1.0)
                        .color(theme.foreground)
                        .font(font)
                        .size(font_size),
                ]
                .spacing(5)
            );
            
            info_col = info_col.push(
                row![
                    text("Humidity: ")
                        .line_height(1.0)
                        .color(theme.color8)
                        .font(font)
                        .size(font_size),
                    text(format!("{}%", weather_clone.humidity))
                        .line_height(1.0)
                        .color(theme.foreground)
                        .font(font)
                        .size(font_size),
                ]
                .spacing(5)
            );
            
            if is_updating {
                info_col = info_col.push(
                    text("↻ updating...")
                        .color(theme.color8)
                        .font(font)
                        .size(font_size * 0.9)
                );
            }
            
            let top_section = row![
                container(art_col)
                    .width(Length::Fixed(160.0))
                    .padding(Padding { top: 10.0, right: 0.0, bottom: 0.0, left: 45.0 }),
                container(info_col)
                    .width(Length::Fixed(200.0))
                    .padding(Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 45.0 }),
            ]
            .spacing(1);
            
            let forecast_lines = Self::format_hourly_forecast(&weather_clone.hourly);
            let mut forecast_col = column![].spacing(2).padding(Padding { top: 20.0, right: 30.0, bottom: 0.0, left: 0.0 });
            
            for (i, line) in forecast_lines.into_iter().enumerate() {
                let color = if i == 0 {
                    theme.color4
                } else {
                    theme.foreground
                };
                
                forecast_col = forecast_col.push(
                    text(line)
                        .line_height(1.0)
                        .color(color)
                        .font(font)
                        .size(font_size * 0.9)
                );
            }
            
            column![
                top_section,
                container(forecast_col)
                    .width(Length::Fill)
                    .center_x(Length::Fill)
            ]
            .spacing(0)
            
        } else if is_updating {
            column![
                container(
                    text("Loading weather...")
                        .color(theme.color8)
                        .font(font)
                        .size(font_size)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
            ]
        } else if let Some(error) = &last_error {
            let error_text = error.clone();
            column![
                container(
                    column![
                        text("⚠ Weather unavailable")
                            .color(theme.color1)
                            .font(font)
                            .size(font_size),
                        text("")
                            .font(font)
                            .size(font_size * 0.5),
                        text(error_text)
                            .color(theme.color8)
                            .font(font)
                            .size(font_size * 0.8),
                        text("")
                            .font(font)
                            .size(font_size * 0.5),
                        text("Check your internet connection")
                            .color(theme.color8)
                            .font(font)
                            .size(font_size * 0.8),
                    ]
                    .spacing(5)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
            ]
        } else {
            column![
                container(
                    text("No weather data")
                        .color(theme.color8)
                        .font(font)
                        .size(font_size)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
            ]
        };

        container(
            container(
                stack![
                    container(
                        container(content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(10)
                            .style(move |_| container::Style {
                                background: None,
                                border: Border {
                                    color: theme.color3,
                                    width: 2.0,
                                    radius: 0.0.into(),
                                },
                                ..Default::default()
                            })
                    )
                    .padding(Padding { top: 15.0, right: 0.0, bottom: 0.0, left: 0.0 })
                    .width(Length::Fill)
                    .height(Length::Fill),
                    
                    container(
                        container(
                            text(" Weather ")
                                .color(theme.color6)
                                .font(font)
                                .size(font_size)
                        )
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .style(move |_| container::Style {
                            background: Some(bg_with_alpha.into()),
                            ..Default::default()
                        })
                    )
                    .padding(Padding { top: 5.0, right: 0.0, bottom: 0.0, left: 8.0 })
                    .width(Length::Shrink)
                    .height(Length::Shrink),
                ]
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: None,
                ..Default::default()
            })
        )
        .width(Length::Fill)
        .height(Length::FillPortion(1))
        .style(move |_| container::Style {
            background: None,
            ..Default::default()
        })
        .into()
    }
}

impl Default for WeatherPanel {
    fn default() -> Self {
        Self::new()
    }
}
