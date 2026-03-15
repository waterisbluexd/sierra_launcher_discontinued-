use iced::widget::{container, text, column, stack, row, button};
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
        let inactive_accent = theme.color8;
        
        let wifi_text_color = if wifi_enabled { theme.color2 } else { inactive_accent };
        let wifi_icon_str = if wifi_enabled { "󰤨" } else { "󰤮" };

        let status_label = if is_connected {
            format!("Connected: {}", wifi_name)
        } else if wifi_enabled {
            "No Network".to_string()
        } else {
            "WiFi Disabled".to_string()
        };

        let wifi_content = column![
            // Centered icon + status info
            container(
                column![
                    text(wifi_icon_str)
                        .font(font)
                        .size(font_size * 4.0)
                        .color(wifi_text_color)
                        .center(),
                    container(text("")).height(Length::Fixed(12.0)),
                    text(if wifi_enabled { "WiFi On" } else { "WiFi Off" })
                        .font(font)
                        .size(font_size * 1.2)
                        .color(wifi_text_color)
                        .center(),
                    container(text("")).height(Length::Fixed(6.0)),
                    text(status_label)
                        .font(font)
                        .size(font_size * 0.85)
                        .color(Color::from_rgba(
                            theme.color6.r,
                            theme.color6.g,
                            theme.color6.b,
                            0.7,
                        ))
                        .center(),
                ]
                .spacing(0)
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),

            // Back button pinned to bottom
            container(
                button(
                    container(
                        text("← Back to Services")
                            .font(font)
                            .size(font_size * 0.9)
                            .color(theme.color6)
                    )
                    .padding(iced::padding::horizontal(16).vertical(6))
                    .center_x(Length::Shrink)
                    .center_y(Length::Shrink)
                )
                .on_press(Message::GoBackToServices)
                .style(move |_, status| {
                    match status {
                        iced::widget::button::Status::Hovered => button::Style {
                            background: Some(Color::from_rgba(
                                theme.color3.r,
                                theme.color3.g,
                                theme.color3.b,
                                0.3,
                            ).into()),
                            border: Border {
                                color: theme.color6,
                                width: 1.5,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        },
                        _ => button::Style {
                            background: Some(Color::TRANSPARENT.into()),
                            border: Border {
                                color: theme.color3,
                                width: 1.5,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                }),
            )
            .width(Length::Fill)
            .padding(iced::padding::bottom(12))
            .center_x(Length::Fill),
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center);

        container(
            container(
                stack![
                    container(
                        container(wifi_content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(iced::padding::top(25).left(10).right(10).bottom(10))
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

                    // Floating title label — same pattern as clock, weather, system, etc.
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
