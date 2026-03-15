use iced::widget::{container, text, column, stack, row, scrollable, button};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use crate::panels::system::system_services::{WifiNetwork, fetch_wifi_networks, signal_icon};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const WINDOW_SIZE: usize = 8; // visible rows at a time

pub struct WifiPanel {
    networks: Arc<Mutex<Vec<WifiNetwork>>>,
    pub selected_index: usize,
    window_start: usize,
    last_scan: Arc<Mutex<Instant>>,
    scanning: Arc<Mutex<bool>>,
}

impl WifiPanel {
    pub fn new() -> Self {
        let networks = Arc::new(Mutex::new(Vec::new()));
        let last_scan = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60)));
        let scanning = Arc::new(Mutex::new(false));

        // Initial background scan
        Self::trigger_scan(
            Arc::clone(&networks),
            Arc::clone(&last_scan),
            Arc::clone(&scanning),
        );

        Self {
            networks,
            selected_index: 0,
            window_start: 0,
            last_scan,
            scanning,
        }
    }

    fn trigger_scan(
        networks: Arc<Mutex<Vec<WifiNetwork>>>,
        last_scan: Arc<Mutex<Instant>>,
        scanning: Arc<Mutex<bool>>,
    ) {
        // Don't double-scan
        if *scanning.lock().unwrap() {
            return;
        }
        *scanning.lock().unwrap() = true;

        std::thread::spawn(move || {
            let result = fetch_wifi_networks();
            if let Ok(mut nets) = networks.lock() {
                *nets = result;
            }
            if let Ok(mut t) = last_scan.lock() {
                *t = Instant::now();
            }
            if let Ok(mut s) = scanning.lock() {
                *s = false;
            }
        });
    }

    /// Called from update on WifiScanRefresh — rescans if >10s since last scan
    pub fn maybe_rescan(&self) {
        let elapsed = self.last_scan.lock()
            .map(|t| t.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if elapsed > Duration::from_secs(10) {
            Self::trigger_scan(
                Arc::clone(&self.networks),
                Arc::clone(&self.last_scan),
                Arc::clone(&self.scanning),
            );
        }
    }

    /// Force an immediate rescan regardless of elapsed time
    pub fn force_rescan(&self) {
        Self::trigger_scan(
            Arc::clone(&self.networks),
            Arc::clone(&self.last_scan),
            Arc::clone(&self.scanning),
        );
    }

    pub fn network_count(&self) -> usize {
        self.networks.lock().map(|n| n.len()).unwrap_or(0)
    }

    pub fn arrow_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.update_window();
        }
    }

    pub fn arrow_down(&mut self) {
        let count = self.network_count();
        if count > 0 && self.selected_index + 1 < count {
            self.selected_index += 1;
            self.update_window();
        }
    }

    fn update_window(&mut self) {
        let count = self.network_count();
        if count == 0 {
            self.window_start = 0;
            return;
        }
        if self.selected_index >= self.window_start + WINDOW_SIZE {
            self.window_start = self.selected_index + 1 - WINDOW_SIZE;
        } else if self.selected_index < self.window_start {
            self.window_start = self.selected_index;
        }
    }

    pub fn is_scanning(&self) -> bool {
        *self.scanning.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn view<'a>(
        &'a self,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let networks = self.networks.lock()
            .map(|n| n.clone())
            .unwrap_or_default();

        let scanning = self.is_scanning();

        // ── Build the list ────────────────────────────────────────────────
        let mut list = column![].spacing(1);

        // Small top padding
        list = list.push(container(text("")).height(Length::Fixed(4.0)));

        if networks.is_empty() {
            if scanning {
                list = list.push(
                    container(
                        text("Scanning…")
                            .font(font)
                            .size(font_size * 0.9)
                            .color(Color::from_rgba(
                                theme.color6.r, theme.color6.g, theme.color6.b, 0.5,
                            ))
                    )
                    .padding(iced::padding::horizontal(10).vertical(8))
                    .width(Length::Fill)
                    .center_x(Length::Fill),
                );
            } else {
                list = list.push(
                    container(
                        text("No networks found")
                            .font(font)
                            .size(font_size * 0.9)
                            .color(Color::from_rgba(
                                theme.color6.r, theme.color6.g, theme.color6.b, 0.5,
                            ))
                    )
                    .padding(iced::padding::horizontal(10).vertical(8))
                    .width(Length::Fill)
                    .center_x(Length::Fill),
                );
            }
        } else {
            let window_end = (self.window_start + WINDOW_SIZE).min(networks.len());

            for idx in self.window_start..window_end {
                let net = networks[idx].clone();
                let selected = idx == self.selected_index;

                // Row background
                let row_bg = if selected {
                    Some(Color::from_rgba(
                        theme.color3.r, theme.color3.g, theme.color3.b, 0.5,
                    ).into())
                } else if idx % 2 == 0 {
                    Some(Color::from_rgba(
                        theme.color0.r, theme.color0.g, theme.color0.b, 0.08,
                    ).into())
                } else {
                    None
                };

                let text_color = if selected {
                    theme.foreground
                } else if net.connected {
                    theme.color2
                } else {
                    theme.color6
                };

                let signal_color = if net.connected {
                    theme.color2
                } else {
                    signal_strength_color(theme, net.signal)
                };

                let arrow        = if selected { ">>" } else { "  " };
                let conn_marker  = if net.connected { "✓ " } else { "  " };
                let lock_icon    = if net.secured { " 󰌾" } else { "  " };
                let sig_icon     = signal_icon(net.signal);

                let net_row = row![
                    // selector arrow
                    text(arrow)
                        .font(font)
                        .size(font_size * 0.85)
                        .color(text_color)
                        .width(Length::Fixed(22.0)),

                    // connected checkmark
                    text(conn_marker)
                        .font(font)
                        .size(font_size * 0.85)
                        .color(theme.color2)
                        .width(Length::Fixed(18.0)),

                    // SSID — max 30 chars, fills remaining space
                    text(net.ssid.clone())
                        .font(font)
                        .size(font_size * 0.9)
                        .color(text_color)
                        .width(Length::Fill),

                    // lock icon
                    text(lock_icon)
                        .font(font)
                        .size(font_size * 0.85)
                        .color(Color::from_rgba(
                            theme.color6.r, theme.color6.g, theme.color6.b, 0.5,
                        ))
                        .width(Length::Fixed(22.0)),

                    // signal strength icon
                    text(sig_icon)
                        .font(font)
                        .size(font_size * 1.1)
                        .color(signal_color)
                        .width(Length::Fixed(22.0)),

                    // signal percentage
                    text(format!("{:>3}%", net.signal))
                        .font(font)
                        .size(font_size * 0.8)
                        .color(Color::from_rgba(
                            theme.color6.r, theme.color6.g, theme.color6.b, 0.6,
                        ))
                        .width(Length::Fixed(36.0)),
                ]
                .spacing(2)
                .align_y(alignment::Vertical::Center)
                .width(Length::Fill);

                // Thin divider under each row
                let divider = container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fixed(1.0))
                    .style(move |_| container::Style {
                        background: Some(Color::from_rgba(
                            theme.color6.r, theme.color6.g, theme.color6.b, 0.1,
                        ).into()),
                        ..Default::default()
                    });

                list = list.push(
                    container(
                        column![net_row, divider].spacing(0)
                    )
                    .padding(iced::padding::horizontal(8).vertical(5))
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: row_bg,
                        border: Border::default(),
                        ..Default::default()
                    }),
                );
            }

            // Position indicator when list is longer than window
            if networks.len() > WINDOW_SIZE {
                let shown_end = (self.window_start + WINDOW_SIZE).min(networks.len());
                list = list.push(
                    container(
                        text(format!(
                            "{}-{} / {}",
                            self.window_start + 1,
                            shown_end,
                            networks.len()
                        ))
                        .font(font)
                        .size(font_size * 0.75)
                        .color(Color::from_rgba(
                            theme.color6.r, theme.color6.g, theme.color6.b, 0.4,
                        ))
                    )
                    .padding(iced::padding::horizontal(10).vertical(4))
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Right),
                );
            }
        }

        // ── Column header ─────────────────────────────────────────────────
        let header = row![
            // space for arrow + checkmark columns
            text("  ")
                .font(font)
                .size(font_size * 0.72)
                .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.4))
                .width(Length::Fixed(40.0)),
            text("NETWORK")
                .font(font)
                .size(font_size * 0.72)
                .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.4))
                .width(Length::Fill),
            text("SIGNAL")
                .font(font)
                .size(font_size * 0.72)
                .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.4))
                .width(Length::Fixed(76.0)),
        ]
        .spacing(2)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill);

        let header_divider = container(text(""))
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(move |_| container::Style {
                background: Some(Color::from_rgba(
                    theme.color3.r, theme.color3.g, theme.color3.b, 0.6,
                ).into()),
                ..Default::default()
            });

        // Scanning indicator shown bottom-left
        let scan_indicator: Element<'a, Message> = if scanning {
            text("↻ scanning…")
                .font(font)
                .size(font_size * 0.75)
                .color(Color::from_rgba(
                    theme.color6.r, theme.color6.g, theme.color6.b, 0.45,
                ))
                .into()
        } else {
            text("")
                .font(font)
                .size(font_size * 0.75)
                .into()
        };

        // ── Back button ───────────────────────────────────────────────────
        let back_button = button(
            container(
                text("← Back")
                    .font(font)
                    .size(font_size * 0.85)
                    .color(theme.color6)
            )
            .padding(iced::padding::horizontal(12).vertical(4))
            .center_x(Length::Shrink)
            .center_y(Length::Shrink)
        )
        .on_press(Message::GoBackToServices)
        .style(move |_, status| {
            match status {
                iced::widget::button::Status::Hovered => iced::widget::button::Style {
                    background: Some(Color::from_rgba(
                        theme.color3.r, theme.color3.g, theme.color3.b, 0.3,
                    ).into()),
                    border: Border { color: theme.color6, width: 1.5, radius: 0.0.into() },
                    ..Default::default()
                },
                _ => iced::widget::button::Style {
                    background: Some(Color::TRANSPARENT.into()),
                    border: Border { color: theme.color3, width: 1.5, radius: 0.0.into() },
                    ..Default::default()
                }
            }
        });

        // ── Full inner content ────────────────────────────────────────────
        let inner_content = column![
            container(header)
                .padding(iced::padding::horizontal(8).vertical(4))
                .width(Length::Fill),
            header_divider,
            // scrollable list takes all available space
            scrollable(list)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_, _| scrollable::Style {
                    container: container::Style::default(),
                    vertical_rail: scrollable::Rail {
                        background: None,
                        border: Border::default(),
                        scroller: scrollable::Scroller {
                            background: iced::Background::Color(Color::from_rgba(
                                0.5, 0.5, 0.5, 0.2,
                            )),
                            border: Border::default(),
                        },
                    },
                    horizontal_rail: scrollable::Rail {
                        background: None,
                        border: Border::default(),
                        scroller: scrollable::Scroller {
                            background: iced::Background::Color(Color::TRANSPARENT),
                            border: Border::default(),
                        },
                    },
                    gap: None,
                    auto_scroll: scrollable::AutoScroll {
                        background: iced::Background::Color(Color::TRANSPARENT),
                        border: Border::default(),
                        icon: Color::TRANSPARENT,
                        shadow: iced::Shadow::default(),
                    },
                }),
            // bottom bar: scan indicator left, back button right
            container(
                row![
                    container(scan_indicator)
                        .width(Length::Fill)
                        .align_y(alignment::Vertical::Center),
                    container(back_button)
                        .width(Length::Shrink)
                        .align_y(alignment::Vertical::Center),
                ]
                .align_y(alignment::Vertical::Center)
                .width(Length::Fill)
            )
            .padding(iced::padding::horizontal(8).vertical(4))
            .width(Length::Fill),
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        // ── Outer panel frame — same structure as clock, weather, system ──
        container(
            container(
                stack![
                    container(
                        container(inner_content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(iced::padding::top(25).left(6).right(6).bottom(6))
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

                    // Floating " Wifi " title label cutting into the border
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

/// Map signal strength to a theme color: green → yellow → red
fn signal_strength_color(theme: &Theme, signal: u8) -> Color {
    match signal {
        75..=100 => theme.color2, // strong  → green
        50..=74  => theme.color3, // good    → yellow
        25..=49  => theme.color3, // fair    → yellow
        _        => theme.color1, // weak    → red
    }
}

impl Default for WifiPanel {
    fn default() -> Self {
        Self::new()
    }
}
