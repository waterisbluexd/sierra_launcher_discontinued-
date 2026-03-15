use iced::widget::{container, text, column, stack, row, button, text_input};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use crate::panels::system::system_services::{WifiNetwork, fetch_wifi_networks, signal_icon};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const WINDOW_SIZE: usize = 4;

#[derive(Debug, Clone)]
pub struct ConnectPrompt {
    pub ssid: String,
    pub secured: bool,
    pub password: String,
    pub input_id: iced::widget::Id,
    pub show_password: bool,
}

pub struct WifiPanel {
    networks: Arc<Mutex<Vec<WifiNetwork>>>,
    pub selected_index: usize,
    window_start: usize,
    last_scan: Arc<Mutex<Instant>>,
    scanning: Arc<Mutex<bool>>,
    pub connect_prompt: Option<ConnectPrompt>,
}

impl WifiPanel {
    pub fn new() -> Self {
        let networks = Arc::new(Mutex::new(Vec::new()));
        let last_scan = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60)));
        let scanning  = Arc::new(Mutex::new(false));
        Self::trigger_scan(Arc::clone(&networks), Arc::clone(&last_scan), Arc::clone(&scanning));
        Self { networks, selected_index: 0, window_start: 0, last_scan, scanning, connect_prompt: None }
    }

    fn trigger_scan(
        networks: Arc<Mutex<Vec<WifiNetwork>>>,
        last_scan: Arc<Mutex<Instant>>,
        scanning: Arc<Mutex<bool>>,
    ) {
        if *scanning.lock().unwrap() { return; }
        *scanning.lock().unwrap() = true;
        std::thread::spawn(move || {
            let result = fetch_wifi_networks();
            if let Ok(mut n) = networks.lock() { *n = result; }
            if let Ok(mut t) = last_scan.lock() { *t = Instant::now(); }
            if let Ok(mut s) = scanning.lock()  { *s = false; }
        });
    }

    pub fn maybe_rescan(&self) {
        let elapsed = self.last_scan.lock().map(|t| t.elapsed()).unwrap_or(Duration::from_secs(0));
        if elapsed > Duration::from_secs(10) {
            Self::trigger_scan(Arc::clone(&self.networks), Arc::clone(&self.last_scan), Arc::clone(&self.scanning));
        }
    }

    pub fn force_rescan(&self) {
        Self::trigger_scan(Arc::clone(&self.networks), Arc::clone(&self.last_scan), Arc::clone(&self.scanning));
    }

    pub fn network_count(&self) -> usize {
        self.networks.lock().map(|n| n.len()).unwrap_or(0)
    }

    pub fn arrow_up(&mut self) {
        if self.connect_prompt.is_some() { return; }
        if self.selected_index > 0 { self.selected_index -= 1; self.update_window(); }
    }

    pub fn arrow_down(&mut self) {
        if self.connect_prompt.is_some() { return; }
        let count = self.network_count();
        if count > 0 && self.selected_index + 1 < count { self.selected_index += 1; self.update_window(); }
    }

    fn update_window(&mut self) {
        let count = self.network_count();
        if count == 0 { self.window_start = 0; return; }
        if self.selected_index >= self.window_start + WINDOW_SIZE {
            self.window_start = self.selected_index + 1 - WINDOW_SIZE;
        } else if self.selected_index < self.window_start {
            self.window_start = self.selected_index;
        }
    }

    pub fn open_connect_prompt(&mut self) {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        if let Some(net) = networks.get(self.selected_index) {
            self.connect_prompt = Some(ConnectPrompt {
                ssid: net.ssid.clone(),
                secured: net.secured,
                password: String::new(),
                input_id: iced::widget::Id::unique(),
                show_password: false,
            });
        }
    }

    pub fn close_connect_prompt(&mut self) { self.connect_prompt = None; }

    pub fn set_password(&mut self, pw: String) {
        if let Some(ref mut p) = self.connect_prompt { p.password = pw; }
    }

    pub fn toggle_show_password(&mut self) {
        if let Some(ref mut p) = self.connect_prompt { p.show_password = !p.show_password; }
    }

    pub fn take_connect_action(&mut self) -> Option<(String, String)> {
        self.connect_prompt.take().map(|p| (p.ssid, p.password))
    }

    pub fn is_scanning(&self) -> bool {
        *self.scanning.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ── view ─────────────────────────────────────────────────────────────

    pub fn view<'a>(&'a self, theme: &'a Theme, bg_with_alpha: Color, font: iced::Font, font_size: f32) -> Element<'a, Message> {
        if let Some(ref prompt) = self.connect_prompt {
            return self.view_connect_prompt(prompt, theme, bg_with_alpha, font, font_size);
        }
        self.view_list(theme, bg_with_alpha, font, font_size)
    }

    // ── list view ────────────────────────────────────────────────────────

    fn view_list<'a>(&'a self, theme: &'a Theme, bg_with_alpha: Color, font: iced::Font, font_size: f32) -> Element<'a, Message> {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        let scanning = self.is_scanning();
        let rfs = font_size * 0.9;
        let dim = Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.35);

        // header
        let header = container(
            row![
                text("").width(Length::Fixed(30.0)),
                text("NETWORK").font(font).size(font_size * 0.68).color(dim).width(Length::Fill),
                text("SIG").font(font).size(font_size * 0.68).color(dim).width(Length::Fixed(24.0)),
            ]
            .spacing(2).align_y(alignment::Vertical::Center)
        )
        .padding(iced::padding::horizontal(8).vertical(2))
        .width(Length::Fill);

        // rows
        let mut rows = column![].spacing(0);

        if networks.is_empty() {
            let msg = if scanning { "Scanning for networks…" } else { "No networks found" };
            rows = rows.push(
                container(text(msg).font(font).size(rfs)
                    .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.45)))
                .padding(iced::padding::horizontal(12).vertical(12))
                .width(Length::Fill),
            );
        } else {
            let window_end = (self.window_start + WINDOW_SIZE).min(networks.len());
            for idx in self.window_start..window_end {
                let net = &networks[idx];
                let selected = idx == self.selected_index;

                let row_bg: Option<iced::Background> = if selected {
                    Some(Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.45).into())
                } else if idx % 2 == 0 {
                    Some(Color::from_rgba(theme.color0.r, theme.color0.g, theme.color0.b, 0.10).into())
                } else { None };

                let ssid_color = if selected { theme.foreground } else if net.connected { theme.color2 } else { theme.color6 };
                let sig_color = sig_color(theme, net);

                let net_row = row![
                    text(if selected { ">>" } else { "  " }).font(font).size(rfs).color(ssid_color).width(Length::Fixed(20.0)),
                    text(if net.connected { "✓" } else { " " }).font(font).size(rfs).color(theme.color2).width(Length::Fixed(14.0)),
                    text(net.ssid.clone()).font(font).size(rfs).color(ssid_color).width(Length::Fill),
                    text(if net.secured { " 󰌾" } else { " " }).font(font).size(rfs * 0.85)
                        .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.4))
                        .width(Length::Fixed(16.0)),
                    text(signal_icon(net.signal)).font(font).size(rfs * 1.05).color(sig_color).width(Length::Fixed(16.0)),
                ]
                .spacing(2).align_y(alignment::Vertical::Center).width(Length::Fill);

                let sep = container(text("")).width(Length::Fill).height(Length::Fixed(1.0))
                    .style(move |_| container::Style {
                        background: Some(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.07).into()),
                        ..Default::default()
                    });

                rows = rows.push(
                    container(column![net_row, sep].spacing(0))
                        .padding(iced::padding::horizontal(8).vertical(4))
                        .width(Length::Fill)
                        .style(move |_| container::Style { background: row_bg, ..Default::default() }),
                );
            }
        }

        // bottom bar
        let pos_txt = if networks.len() > WINDOW_SIZE {
            let end = (self.window_start + WINDOW_SIZE).min(networks.len());
            format!("{}-{}/{}", self.window_start + 1, end, networks.len())
        } else { String::new() };
        let scan_txt = if scanning { "↻" } else { "" };
        let status = [scan_txt, pos_txt.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("  ");

        let bottom_bar = container(
            row![
                pill_btn("← Back", theme, font, font_size * 0.88).on_press(Message::GoBackToServices),
                pill_btn("↻ Scan", theme, font, font_size * 0.88).on_press(Message::WifiForceScan),
                container(
                    text(status).font(font).size(font_size * 0.72)
                        .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.35))
                ).width(Length::Fill).center_x(Length::Fill).align_y(alignment::Vertical::Center),
                pill_btn("Connect →", theme, font, font_size * 0.88).on_press(Message::WifiOpenConnect),
            ]
            .spacing(6).align_y(alignment::Vertical::Center).width(Length::Fill)
        )
        .padding(iced::padding::horizontal(8).vertical(6))
        .width(Length::Fill);

        let inner = column![header, hairline(theme), container(rows).width(Length::Fill).height(Length::Fill), hairline(theme), bottom_bar]
            .spacing(0).width(Length::Fill).height(Length::Fill);

        panel_frame(inner, theme, bg_with_alpha, font, font_size, " Wifi ")
    }

    // ── connect prompt ───────────────────────────────────────────────────

    fn view_connect_prompt<'a>(
        &'a self,
        prompt: &'a ConnectPrompt,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let dim = Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.45);

        // ── SSID row — just the name, clean ──────────────────────────────
        let ssid_section = container(
            row![
                text(signal_icon(100)).font(font).size(font_size * 1.1).color(theme.color2),
                text("  "),
                text(&prompt.ssid).font(font).size(font_size * 1.05).color(theme.color6),
                container(text("")).width(Length::Fill),
                // lock badge if secured
                if prompt.secured {
                    text(" 󰌾 secured").font(font).size(font_size * 0.75)
                        .color(dim)
                } else {
                    text(" 󰤨 open").font(font).size(font_size * 0.75)
                        .color(theme.color2)
                },
            ]
            .spacing(0)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
        )
        .padding(iced::padding::horizontal(12).vertical(12))
        .width(Length::Fill);

        // ── Password section (only if secured) ───────────────────────────
        let body: Element<'a, Message> = if prompt.secured {
            // Plain text input (password masking not available in this Iced version)
            let input_style = move |_: &_, _: _| text_input::Style {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
                icon: Color::TRANSPARENT,
                placeholder: Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.25),
                value: theme.color6,
                selection: theme.color4,
            };

            let pw_input: Element<'a, Message> = text_input("", &prompt.password)
                .on_input(Message::WifiPasswordInput)
                .on_submit(Message::WifiDoConnect)
                .id(prompt.input_id.clone())
                .font(font)
                .size(font_size)
                .padding(iced::padding::horizontal(10).vertical(10))
                .style(input_style)
                .into();

            // Eye toggle button — same size as the input text
            let eye_icon = if prompt.show_password { "󰛐" } else { "󰛑" };
            let eye_btn = button(
                container(
                    text(eye_icon).font(font).size(font_size).color(
                        Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.7)
                    )
                )
                .width(Length::Fixed(36.0))
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
            )
            .on_press(Message::WifiTogglePasswordVisibility)
            .padding(0)
            .style(move |_, status| match status {
                iced::widget::button::Status::Hovered => button::Style {
                    background: Some(Color::from_rgba(
                        theme.color3.r, theme.color3.g, theme.color3.b, 0.25,
                    ).into()),
                    ..Default::default()
                },
                _ => button::Style {
                    background: Some(Color::TRANSPARENT.into()),
                    ..Default::default()
                },
            });

            // Thin vertical divider between input and eye button
            let vdivider = container(text(""))
                .width(Length::Fixed(1.0))
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color::from_rgba(
                        theme.color3.r, theme.color3.g, theme.color3.b, 0.5,
                    ).into()),
                    ..Default::default()
                });

            // Input row: [  input field  | | eye ]
            let input_row = row![
                container(pw_input).width(Length::Fill).height(Length::Fill),
                vdivider,
                eye_btn,
            ]
            .spacing(0)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill);

            // Label above the row
            let pw_label = text(" password ")
                .font(font)
                .size(font_size)
                .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.55));

            container(
                column![
                    container(pw_label)
                        .padding(iced::padding::horizontal(12).top(14).bottom(4))
                        .width(Length::Fill),
                    container(input_row)
                        .width(Length::Fill)
                        .height(Length::Fixed(46.0))
                        .padding(iced::padding::horizontal(12).vertical(0))
                        .style(move |_| container::Style { background: None, ..Default::default() }),
                ]
                .spacing(0)
            )
            .width(Length::Fill)
            .into()
        } else {
            // Open network — no password needed
            container(
                row![
                    text("󰤨").font(font).size(font_size * 1.1).color(theme.color2),
                    text("  Open network — no password required").font(font).size(font_size * 0.88)
                        .color(dim),
                ]
                .spacing(0)
                .align_y(alignment::Vertical::Center)
            )
            .padding(iced::padding::horizontal(12).vertical(14))
            .width(Length::Fill)
            .into()
        };

        // ── Bottom bar: Cancel | Connect ─────────────────────────────────
        let bottom_bar = container(
            row![
                pill_btn("← Cancel", theme, font, font_size * 0.88).on_press(Message::WifiCloseConnect),
                container(text("")).width(Length::Fill),
                pill_btn("Connect →", theme, font, font_size * 0.88).on_press(Message::WifiDoConnect),
            ]
            .spacing(6).align_y(alignment::Vertical::Center).width(Length::Fill)
        )
        .padding(iced::padding::horizontal(8).vertical(6))
        .width(Length::Fill);

        // ── Assemble ─────────────────────────────────────────────────────
        let inner = column![
            ssid_section,
            hairline(theme),
            container(body).width(Length::Fill).height(Length::Fill),
            hairline(theme),
            bottom_bar,
        ]
        .spacing(0).width(Length::Fill).height(Length::Fill);

        panel_frame(inner, theme, bg_with_alpha, font, font_size, " Wifi ")
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn hairline<'a>(theme: &'a Theme) -> container::Container<'a, Message> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_| container::Style {
            background: Some(Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.45).into()),
            ..Default::default()
        })
}

fn pill_btn<'a>(label: &'a str, theme: &'a Theme, font: iced::Font, fs: f32) -> iced::widget::Button<'a, Message> {
    iced::widget::button(text(label).font(font).size(fs).color(theme.color6))
        .padding(iced::padding::horizontal(12).vertical(5))
        .style(move |_, status| match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.35).into()),
                border: Border { color: theme.color6, width: 1.5, radius: 0.0.into() },
                ..Default::default()
            },
            _ => iced::widget::button::Style {
                background: Some(Color::TRANSPARENT.into()),
                border: Border { color: theme.color3, width: 1.5, radius: 0.0.into() },
                ..Default::default()
            },
        })
}

fn sig_color(theme: &Theme, net: &WifiNetwork) -> Color {
    if net.connected { return theme.color2; }
    match net.signal {
        75..=100 => theme.color2,
        50..=74  => theme.color3,
        25..=49  => theme.color3,
        _        => theme.color1,
    }
}

fn panel_frame<'a>(
    inner: impl Into<Element<'a, Message>>,
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    title: &'static str,
) -> Element<'a, Message> {
    container(
        container(
            stack![
                container(
                    container(inner.into())
                        .width(Length::Fill).height(Length::Fill)
                        .padding(iced::padding::top(18).left(6).right(6).bottom(6))
                        .style(move |_| container::Style {
                            background: None,
                            border: Border { color: theme.color3, width: 2.0, radius: 0.0.into() },
                            ..Default::default()
                        })
                )
                .padding(iced::padding::top(10))
                .width(Length::Fill).height(Length::Fill),

                container(
                    container(text(title).color(theme.color6).font(font).size(font_size))
                        .width(Length::Shrink).height(Length::Shrink)
                        .style(move |_| container::Style {
                            background: Some(bg_with_alpha.into()),
                            ..Default::default()
                        })
                )
                .padding(iced::padding::left(8).top(5))
                .width(Length::Shrink).height(Length::Shrink),
            ]
        )
        .width(Length::Fill).height(Length::Fill)
        .style(move |_| container::Style { background: None, ..Default::default() }),
    )
    .width(Length::Fill).height(Length::FillPortion(1))
    .style(move |_| container::Style { background: None, ..Default::default() })
    .into()
}

impl Default for WifiPanel { fn default() -> Self { Self::new() } }
