use iced::widget::{container, text, column, stack, row, button};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use crate::app::state::Launcher;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    
    pub fn toggle_wifi(&self) {
        let is_enabling = !self.wifi_enabled();
        
        std::thread::spawn(move || {
            crate::panels::system::system_services::toggle_wifi_cmd(is_enabling);
        });
    }

    pub fn view<'a>(&'a self, theme: &'a Theme, bg_with_alpha: Color, font: iced::Font, font_size: f32) -> Element<'a, Message> {
        let wifi_enabled = self.wifi_enabled();
        let wifi_name = self.wifi_name();
        
        let is_connected = wifi_enabled && wifi_name != "No Network" && wifi_name != "WiFi Off";
        
        let wifi_icon_str = if wifi_enabled { "󰤨" } else { "󰤮" };
        
        let wifi_content = column![
            row![
                container(
                    text(wifi_icon_str)
                        .font(font)
                        .size(font_size * 2.0)
                )
                .width(Length::Shrink),
                column![
                    text("CONNECTION")
                        .color(theme.color6)
                        .font(font)
                        .size(font_size * 0.65),
                    text(if wifi_name.len() > 14 {
                        format!("{}..", &wifi_name[..12])
                    } else {
                        wifi_name.clone()
                    })
                        .color(theme.color6)
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
