use iced::widget::{container, text, row, column, Space};
use iced::{Element, Border, Length};
use crate::utils::theme::Theme;
use crate::Message;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;

const BATTERY_SYMBOLS: [&str; 11] = [
    "󰂃", "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹",
];
const BATTERY_CHARGING: &str = "󰂄";
const FAN_SYMBOL: &str = "󰈐";
const TEMP_SYMBOL: &str = "󰔄";

struct SystemCache {
    battery_percent: u8,
    battery_charging: bool,
    fan_rpm: u16,
    cpu_temp: u16,
    last_update: Instant,
}

impl Default for SystemCache {
    fn default() -> Self {
        Self {
            battery_percent: 75,
            battery_charging: false,
            fan_rpm: 1800,
            cpu_temp: 45,
            last_update: Instant::now() - Duration::from_secs(10),
        }
    }
}

lazy_static::lazy_static! {
    static ref SYSTEM_CACHE: Arc<Mutex<SystemCache>> = {
        let cache = Arc::new(Mutex::new(SystemCache::default()));
        let cache_clone = Arc::clone(&cache);
        
        thread::spawn(move || {
            loop {
                let (batt_pct, batt_chrg) = fetch_battery_data();
                let fan = fetch_fan_rpm();
                let temp = fetch_cpu_temp();
                
                if let Ok(mut c) = cache_clone.lock() {
                    c.battery_percent = batt_pct;
                    c.battery_charging = batt_chrg;
                    c.fan_rpm = fan;
                    c.cpu_temp = temp;
                    c.last_update = Instant::now();
                }
                
                thread::sleep(Duration::from_secs(2));
            }
        });
        
        cache
    };
}

fn fetch_battery_data() -> (u8, bool) {
    let mut percent = 75;
    let mut charging = false;
    
    if let Ok(cap) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
        if let Ok(p) = cap.trim().parse::<u8>() {
            percent = p;
        }
    } else if let Ok(cap) = std::fs::read_to_string("/sys/class/power_supply/BAT1/capacity") {
        if let Ok(p) = cap.trim().parse::<u8>() {
            percent = p;
        }
    }
    
    if let Ok(status) = std::fs::read_to_string("/sys/class/power_supply/BAT0/status") {
        charging = status.trim().to_lowercase().contains("charging");
    } else if let Ok(status) = std::fs::read_to_string("/sys/class/power_supply/BAT1/status") {
        charging = status.trim().to_lowercase().contains("charging");
    }
    
    (percent, charging)
}

fn fetch_fan_rpm() -> u16 {
    if let Ok(output) = Command::new("sensors").output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            for line in text.lines() {
                if line.to_lowercase().contains("fan") && line.contains("RPM") {
                    if let Some(rpm_part) = line.split("RPM").next() {
                        if let Some(rpm_str) = rpm_part.split_whitespace().last() {
                            if let Ok(rpm) = rpm_str.parse::<u16>() {
                                return rpm;
                            }
                        }
                    }
                }
            }
        }
    }
    
    for i in 0..10 {
        let fan_path = format!("/sys/class/hwmon/hwmon{}/fan1_input", i);
        if let Ok(rpm_str) = std::fs::read_to_string(&fan_path) {
            if let Ok(rpm) = rpm_str.trim().parse::<u16>() {
                return rpm;
            }
        }
    }
    
    1800
}

fn fetch_cpu_temp() -> u16 {
    for i in 0..10 {
        let temp_path = format!("/sys/class/hwmon/hwmon{}/temp1_input", i);
        if let Ok(temp_str) = std::fs::read_to_string(&temp_path) {
            if let Ok(temp_millidegrees) = temp_str.trim().parse::<u32>() {
                return (temp_millidegrees / 1000) as u16;
            }
        }
    }
    
    if let Ok(output) = Command::new("sensors").output() {
        if let Ok(text) = String::from_utf8(output.stdout) {
            for line in text.lines() {
                if line.contains("Package id") || line.contains("Core 0") {
                    if let Some(temp_part) = line.split('+').nth(1) {
                        if let Some(temp_str) = temp_part.split('°').next() {
                            if let Ok(temp) = temp_str.trim().parse::<f32>() {
                                return temp as u16;
                            }
                        }
                    }
                }
            }
        }
    }
    
    45
}

fn get_battery_symbol(percentage: u8) -> &'static str {
    let index = (percentage / 10).min(10) as usize;
    BATTERY_SYMBOLS[index]
}

fn battery_widget<'a>(
    percentage: u8,
    charging: bool,
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
) -> Element<'a, Message> {
    let symbol = if charging {
        BATTERY_CHARGING
    } else {
        get_battery_symbol(percentage)
    };
    
    let icon_color = if charging {
        theme.color2
    } else if percentage <= 20 {
        theme.color1
    } else if percentage <= 50 {
        theme.color3
    } else {
        theme.color2
    };

    container(
        row![
            text(symbol)
                .font(font)
                .center()
                .size(font_size * 1.6)
                .color(icon_color),
            column![
                text("BATTERY")
                    .center()
                    .font(font)
                    .size(font_size * 0.6)
                    .color(theme.color6),
                text(format!("{}%", percentage))
                    .font(font)
                    .size(font_size * 0.85)
                    .color(theme.color6),
            ]
            .spacing(1)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .padding(iced::padding::left(8).right(6))
    )
    .style(move |_| container::Style {
        background: None,
        border: Border {
            color: theme.color3,
            width: 1.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fixed(45.0))
    .align_y(iced::Alignment::Center)
    .into()
}

fn fan_widget<'a>(
    rpm: u16,
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
) -> Element<'a, Message> {
    let icon_color = if rpm > 3000 {
        theme.color1
    } else if rpm > 1500 {
        theme.color3
    } else {
        theme.color2
    };

    container(
        row![
            text(FAN_SYMBOL)
                .font(font)
                .size(font_size * 1.7)
                .color(icon_color),
            column![
                text("FAN RPM")
                    .font(font)
                    .size(font_size * 0.6)
                    .color(theme.color6),
                text(format!("{}", rpm))
                    .font(font)
                    .size(font_size * 0.85)
                    .color(theme.color6),
            ]
            .spacing(1)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .padding(iced::padding::left(8).right(6))
    )
    .style(move |_| container::Style {
        background: None,
        border: Border {
            color: theme.color3,
            width: 1.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fixed(45.0))
    .align_y(iced::Alignment::Center)
    .into()
}

fn cpu_temp_widget<'a>(
    temp: u16,
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
) -> Element<'a, Message> {
    let icon_color = if temp > 80 {
        theme.color1
    } else if temp > 60 {
        theme.color3
    } else {
        theme.color2
    };

    container(
        row![
            text(TEMP_SYMBOL)
                .font(font)
                .size(font_size * 1.7)
                .color(icon_color),
            column![
                text("CPU TEMP")
                    .font(font)
                    .size(font_size * 0.6)
                    .color(theme.color6),
                text(format!("{}°C", temp))
                    .font(font)
                    .size(font_size * 0.85)
                    .color(theme.color6),
            ]
            .spacing(1)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .padding(iced::padding::left(8).right(6))
    )
    .style(move |_| container::Style {
        background: None,
        border: Border {
            color: theme.color3,
            width: 1.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .height(Length::Fixed(45.0))
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn view_bottom_row<'a>(
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
) -> Element<'a, Message> {
    let (battery_percent, battery_charging, fan_rpm, cpu_temp) = SYSTEM_CACHE
        .lock()
        .map(|c| (c.battery_percent, c.battery_charging, c.fan_rpm, c.cpu_temp))
        .unwrap_or((75, false, 1800, 45));

    container(
        column![
            container(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fixed(1.0))
            )
            .style(move |_| container::Style {
                background: None,
                border: Border {
                    color: theme.color3,
                    width: 1.5,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .width(Length::Fill),
            
            row![
                battery_widget(battery_percent, battery_charging, theme, font, font_size),
                fan_widget(fan_rpm, theme, font, font_size),
                cpu_temp_widget(cpu_temp, theme, font, font_size),
            ]
            .spacing(8)
        ]
        .spacing(5)
        .width(Length::Fill)
    )
    .width(Length::Fill)
    .into()
}
