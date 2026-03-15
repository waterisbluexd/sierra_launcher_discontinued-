use iced::widget::{container, text, column, stack, row, button, text_input};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use crate::panels::system::system_services::{WifiNetwork, fetch_wifi_networks, signal_icon};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rows shown in the list at once. Keep small so rows never clip.
const WINDOW_SIZE: usize = 4;

/// Sub-state for the connect/password prompt overlay.
#[derive(Debug, Clone)]
pub struct ConnectPrompt {
    pub ssid: String,
    pub secured: bool,
    pub password: String,
    pub input_id: iced::widget::Id,
}

pub struct WifiPanel {
    networks: Arc<Mutex<Vec<WifiNetwork>>>,
    pub selected_index: usize,
    window_start: usize,
    last_scan: Arc<Mutex<Instant>>,
    scanning: Arc<Mutex<bool>>,
    /// Some(_) while the connect prompt is open, None otherwise.
    pub connect_prompt: Option<ConnectPrompt>,
}

impl WifiPanel {
    pub fn new() -> Self {
        let networks = Arc::new(Mutex::new(Vec::new()));
        let last_scan = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60)));
        let scanning  = Arc::new(Mutex::new(false));

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
            connect_prompt: None,
        }
    }

    // ── scan ─────────────────────────────────────────────────────────────

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
        let elapsed = self.last_scan.lock()
            .map(|t| t.elapsed()).unwrap_or(Duration::from_secs(0));
        if elapsed > Duration::from_secs(10) {
            Self::trigger_scan(Arc::clone(&self.networks), Arc::clone(&self.last_scan), Arc::clone(&self.scanning));
        }
    }

    pub fn force_rescan(&self) {
        Self::trigger_scan(Arc::clone(&self.networks), Arc::clone(&self.last_scan), Arc::clone(&self.scanning));
    }

    // ── navigation ───────────────────────────────────────────────────────

    pub fn network_count(&self) -> usize {
        self.networks.lock().map(|n| n.len()).unwrap_or(0)
    }

    pub fn arrow_up(&mut self) {
        // If prompt is open, ignore list navigation
        if self.connect_prompt.is_some() { return; }
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.update_window();
        }
    }

    pub fn arrow_down(&mut self) {
        if self.connect_prompt.is_some() { return; }
        let count = self.network_count();
        if count > 0 && self.selected_index + 1 < count {
            self.selected_index += 1;
            self.update_window();
        }
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

    // ── connect prompt ───────────────────────────────────────────────────

    /// Open the connect prompt for the currently selected network.
    pub fn open_connect_prompt(&mut self) {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        if let Some(net) = networks.get(self.selected_index) {
            self.connect_prompt = Some(ConnectPrompt {
                ssid: net.ssid.clone(),
                secured: net.secured,
                password: String::new(),
                input_id: iced::widget::Id::unique(),
            });
        }
    }

    pub fn close_connect_prompt(&mut self) {
        self.connect_prompt = None;
    }

    pub fn set_password(&mut self, pw: String) {
        if let Some(ref mut p) = self.connect_prompt {
            p.password = pw;
        }
    }

    /// Returns (ssid, password) if prompt is open, then closes it.
    pub fn take_connect_action(&mut self) -> Option<(String, String)> {
        self.connect_prompt.take().map(|p| (p.ssid, p.password))
    }

    pub fn is_scanning(&self) -> bool {
        *self.scanning.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ── view ─────────────────────────────────────────────────────────────

    pub fn view<'a>(
        &'a self,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        // If the connect prompt is open, show that instead of the list.
        if let Some(ref prompt) = self.connect_prompt {
            return self.view_connect_prompt(prompt, theme, bg_with_alpha, font, font_size);
        }

        self.view_list(theme, bg_with_alpha, font, font_size)
    }

    // ── list view ────────────────────────────────────────────────────────

    fn view_list<'a>(
        &'a self,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        let scanning = self.is_scanning();
        let rfs      = font_size * 0.9; // row font size
        let dim      = Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.35);

        // ── header row ───────────────────────────────────────────────────
        let header = container(
            row![
                text("").width(Length::Fixed(30.0)),   // selector + tick placeholders
                text("NETWORK")
                    .font(font).size(font_size * 0.68).color(dim)
                    .width(Length::Fill),
                text("SIG")
                    .font(font).size(font_size * 0.68).color(dim)
                    .width(Length::Fixed(24.0)),
            ]
            .spacing(2)
            .align_y(alignment::Vertical::Center)
        )
        .padding(iced::padding::horizontal(8).vertical(2))
        .width(Length::Fill);

        let hline = hairline(theme);

        // ── network rows ─────────────────────────────────────────────────
        let mut rows = column![].spacing(0);

        if networks.is_empty() {
            let msg = if scanning { "Scanning for networks…" } else { "No networks found" };
            rows = rows.push(
                container(
                    text(msg).font(font).size(rfs)
                        .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.45))
                )
                .padding(iced::padding::horizontal(12).vertical(12))
                .width(Length::Fill),
            );
        } else {
            let window_end = (self.window_start + WINDOW_SIZE).min(networks.len());

            for idx in self.window_start..window_end {
                let net      = &networks[idx];
                let selected = idx == self.selected_index;

                let row_bg: Option<iced::Background> = if selected {
                    Some(Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.45).into())
                } else if idx % 2 == 0 {
                    Some(Color::from_rgba(theme.color0.r, theme.color0.g, theme.color0.b, 0.10).into())
                } else {
                    None
                };

                let ssid_color = if selected { theme.foreground }
                    else if net.connected { theme.color2 }
                    else { theme.color6 };

                let sig_color = signal_color(theme, net);

                // >> selector
                let sel = text(if selected { ">>" } else { "  " })
                    .font(font).size(rfs).color(ssid_color)
                    .width(Length::Fixed(20.0));

                // ✓ connected tick
                let tick = text(if net.connected { "✓" } else { " " })
                    .font(font).size(rfs).color(theme.color2)
                    .width(Length::Fixed(14.0));

                // SSID — capped at 30 chars upstream
                let ssid_t = text(net.ssid.clone())
                    .font(font).size(rfs).color(ssid_color)
                    .width(Length::Fill);

                // lock icon — compact, no number
                let lock = text(if net.secured { " 󰌾" } else { " " })
                    .font(font).size(rfs * 0.85)
                    .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.4))
                    .width(Length::Fixed(16.0));

                // signal icon only — no percentage
                let sig = text(signal_icon(net.signal))
                    .font(font).size(rfs * 1.05).color(sig_color)
                    .width(Length::Fixed(16.0));

                let net_row = row![sel, tick, ssid_t, lock, sig]
                    .spacing(2)
                    .align_y(alignment::Vertical::Center)
                    .width(Length::Fill);

                // thin row separator
                let sep = container(text(""))
                    .width(Length::Fill).height(Length::Fixed(1.0))
                    .style(move |_| container::Style {
                        background: Some(Color::from_rgba(
                            theme.color6.r, theme.color6.g, theme.color6.b, 0.07,
                        ).into()),
                        ..Default::default()
                    });

                rows = rows.push(
                    container(column![net_row, sep].spacing(0))
                        .padding(iced::padding::horizontal(8).vertical(4))
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: row_bg, ..Default::default()
                        }),
                );
            }
        }

        // ── bottom bar ───────────────────────────────────────────────────
        // Full-width bar: [ ← Back ] [ Scan ] ...status... [ Connect ]

        let back_btn = wide_button("← Back", theme, font, font_size * 0.88)
            .on_press(Message::GoBackToServices);

        let scan_btn = wide_button("↻ Scan", theme, font, font_size * 0.88)
            .on_press(Message::WifiForceScan);

        let connect_btn = wide_button("Connect →", theme, font, font_size * 0.88)
            .on_press(Message::WifiOpenConnect);

        // Status line: scanning badge + position hint
        let status_txt = {
            let pos = if networks.len() > WINDOW_SIZE {
                let end = (self.window_start + WINDOW_SIZE).min(networks.len());
                format!("{}-{}/{}", self.window_start + 1, end, networks.len())
            } else {
                String::new()
            };
            let scan = if scanning { "↻" } else { "" };
            let combined = [scan, pos.as_str()]
                .iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("  ");
            text(combined)
                .font(font).size(font_size * 0.72)
                .color(Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.35))
        };

        let bottom_bar = container(
            row![
                back_btn,
                scan_btn,
                container(status_txt)
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .align_y(alignment::Vertical::Center),
                connect_btn,
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
        )
        .padding(iced::padding::horizontal(8).vertical(6))
        .width(Length::Fill);

        // ── assemble ─────────────────────────────────────────────────────
        let inner = column![
            header,
            hline,
            container(rows).width(Length::Fill).height(Length::Fill),
            hairline(theme),
            bottom_bar,
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        panel_frame(inner, theme, bg_with_alpha, font, font_size, " Wifi ")
    }

    // ── connect prompt view ──────────────────────────────────────────────

    fn view_connect_prompt<'a>(
        &'a self,
        prompt: &'a ConnectPrompt,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let dim = Color::from_rgba(theme.color6.r, theme.color6.g, theme.color6.b, 0.5);

        // SSID display
        let ssid_row = container(
            column![
                text("Connecting to")
                    .font(font).size(font_size * 0.72).color(dim),
                text(&prompt.ssid)
                    .font(font).size(font_size * 1.1).color(theme.color6),
            ]
            .spacing(2)
        )
        .padding(iced::padding::horizontal(12).vertical(10))
        .width(Length::Fill);

        let ssid_sep = hairline(theme);

        // Body — password field or "no password needed"
        let body: Element<'a, Message> = if prompt.secured {
            column![
                container(
                    text("Password")
                        .font(font).size(font_size * 0.78).color(dim)
                )
                .padding(iced::padding::horizontal(12).top(10).bottom(4))
                .width(Length::Fill),

                container(
                    text_input("Enter password…", &prompt.password)
                        .on_input(Message::WifiPasswordInput)
                        .on_submit(Message::WifiDoConnect)
                        .id(prompt.input_id.clone())
                        .font(font)
                        .size(font_size * 0.9)
                        .padding(8)
                        .style(move |_theme, _status| text_input::Style {
                            background: iced::Background::Color(Color::from_rgba(
                                theme.color0.r, theme.color0.g, theme.color0.b, 0.4,
                            )),
                            border: Border {
                                color: theme.color3,
                                width: 1.5,
                                radius: 0.0.into(),
                            },
                            icon: Color::TRANSPARENT,
                            placeholder: Color::from_rgba(
                                theme.color6.r, theme.color6.g, theme.color6.b, 0.4,
                            ),
                            value: theme.color6,
                            selection: theme.color4,
                        })
                )
                .padding(iced::padding::horizontal(12))
                .width(Length::Fill),

                container(
                    text("Press Enter or Connect to join")
                        .font(font).size(font_size * 0.72).color(dim)
                )
                .padding(iced::padding::horizontal(12).top(6))
                .width(Length::Fill),
            ]
            .spacing(0)
            .width(Length::Fill)
            .into()
        } else {
            container(
                column![
                    text("󰤨  Open network")
                        .font(font).size(font_size * 0.95).color(theme.color2),
                    container(text("")).height(Length::Fixed(6.0)),
                    text("No password required")
                        .font(font).size(font_size * 0.78).color(dim),
                ]
                .spacing(0)
            )
            .padding(iced::padding::horizontal(12).vertical(14))
            .width(Length::Fill)
            .into()
        };

        // Bottom bar: [ ← Cancel ]   [ Connect ]
        let cancel_btn = wide_button("← Cancel", theme, font, font_size * 0.88)
            .on_press(Message::WifiCloseConnect);

        let connect_btn = wide_button("Connect →", theme, font, font_size * 0.88)
            .on_press(Message::WifiDoConnect);

        let bottom_bar = container(
            row![
                cancel_btn,
                container(text("")).width(Length::Fill),
                connect_btn,
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill)
        )
        .padding(iced::padding::horizontal(8).vertical(6))
        .width(Length::Fill);

        let inner = column![
            ssid_row,
            ssid_sep,
            container(body).width(Length::Fill).height(Length::Fill),
            hairline(theme),
            bottom_bar,
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        panel_frame(inner, theme, bg_with_alpha, font, font_size, " Wifi ")
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Thin 1-px horizontal separator.
fn hairline<'a>(theme: &'a Theme) -> container::Container<'a, Message> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_| container::Style {
            background: Some(Color::from_rgba(
                theme.color3.r, theme.color3.g, theme.color3.b, 0.45,
            ).into()),
            ..Default::default()
        })
}

/// A button styled like a bordered label — used for Back / Connect.
fn wide_button<'a>(
    label: &'a str,
    theme: &'a Theme,
    font: iced::Font,
    fs: f32,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(
        text(label).font(font).size(fs).color(theme.color6)
    )
    .padding(iced::padding::horizontal(12).vertical(5))
    .style(move |_, status| match status {
        iced::widget::button::Status::Hovered => iced::widget::button::Style {
            background: Some(Color::from_rgba(
                theme.color3.r, theme.color3.g, theme.color3.b, 0.35,
            ).into()),
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

/// Signal color based on strength and whether connected.
fn signal_color(theme: &Theme, net: &WifiNetwork) -> Color {
    if net.connected { return theme.color2; }
    match net.signal {
        75..=100 => theme.color2,
        50..=74  => theme.color3,
        25..=49  => theme.color3,
        _        => theme.color1,
    }
}

/// Wraps content in the standard panel frame (border + floating title label).
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
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(iced::padding::top(18).left(6).right(6).bottom(6))
                        .style(move |_| container::Style {
                            background: None,
                            border: Border { color: theme.color3, width: 2.0, radius: 0.0.into() },
                            ..Default::default()
                        })
                )
                .padding(iced::padding::top(10))
                .width(Length::Fill)
                .height(Length::Fill),

                // floating title cutting into the top border
                container(
                    container(
                        text(title).color(theme.color6).font(font).size(font_size)
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
        .style(move |_| container::Style { background: None, ..Default::default() }),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(1))
    .style(move |_| container::Style { background: None, ..Default::default() })
    .into()
}

impl Default for WifiPanel {
    fn default() -> Self { Self::new() }
}
