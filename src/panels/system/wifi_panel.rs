use iced::widget::{container, text, column, stack, row};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct WifiPanel {
    status_cache: Arc<Mutex<WifiStatus>>,
}

#[derive(Clone, Default)]
struct WifiStatus {
    wifi_enabled: bool,
    wifi_name: String,
}

impl WifiPanel {
    pub fn new() -> Self {
        let status_cache = Arc::new(Mutex::new(WifiStatus::default()));
        
        let cache_clone = Arc::clone(&status_cache);
        
        std::thread::spawn(move || {
            loop {
                let (wifi_enabled, wifi_name) = crate::panels::system::system_services::fetch_wifi_status();
                
                if let Ok(mut status) = cache_clone.lock() {
                    status.wifi_enabled = wifi_enabled;
                    status.wifi_name = wifi_name;
                }
                
                std::thread::sleep(Duration::from_millis(500));
            }
        });
        
        Self { status_cache }
    }
    
    pub fn wifi_enabled(&self) -> bool {
        self.status_cache.lock().map(|s| s.wifi_enabled).unwrap_or(false)
    }
    
    pub fn wifi_name(&self) -> String {
        self.status_cache.lock()
            .map(|s| s.wifi_name.clone())
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    pub fn view<'a>(&'a self, theme: &'a Theme, bg_with_alpha: Color, font: iced::Font, font_size: f32) -> Element<'a, Message> {
        let wifi_enabled = self.wifi_enabled();
        let wifi_name = self.wifi_name();
        
        let is_connected = wifi_enabled && wifi_name != "No Network" && wifi_name != "WiFi Off";
        let active_accent = if is_connected { theme.color2 } else { theme.color3 };
        let inactive_accent = theme.color8;
        
        let (wifi_text_color, _wifi_bg_color, _wifi_border_color) = if wifi_enabled {
            (theme.color0, theme.color2, theme.color2)
        } else {
            (inactive_accent, Color::TRANSPARENT, inactive_accent)
        };
        
        let wifi_icon_str = if wifi_enabled { "󰤨" } else { "󰤮" };
        
        let wifi_content = column![
            row![
                container(
                    text(wifi_icon_str)
                        .font(font)
                        .size(font_size * 2.0)
                        .color(wifi_text_color)
                )
                .width(Length::Shrink),
                column![
                    text("CONNECTION")
                        .color(wifi_text_color)
                        .font(font)
                        .size(font_size * 0.65),
                    text(if wifi_name.len() > 14 {
                        format!("{}..", &wifi_name[..12])
                    } else {
                        wifi_name.clone()
                    })
                        .color(wifi_text_color)
                        .font(font)
                        .size(font_size * 0.9)
                ]
                .spacing(2)
            ]
            .spacing(10),
        ]
        .spacing(20)
        .align_x(alignment::Horizontal::Center)
        .width(Length::Fill)
        .height(Length::Fill);

        container(
            container(
                stack![
                    container(
                        container(wifi_content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(iced::padding::top(25))
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
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
                    .padding(iced::padding::top(15))
                    .width(Length::Fill)
                    .height(Length::Fill),
                    container(
                        container(
                            text(" Wifi ")
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
                    .padding(iced::padding::left(8).top(5))
                    .width(Length::Shrink)
                    .height(Length::Shrink),
                ]
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: None,
                ..Default::default()
            }),
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

impl Default for WifiPanel {
    fn default() -> Self {
        Self::new()
    }
}
