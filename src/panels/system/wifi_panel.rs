use crate::panels::system::system_services::{fetch_wifi_networks, signal_icon, WifiNetwork};
use crate::utils::theme::Theme;
use crate::utils::wifi_credentials;
use crate::Message;
use iced::widget::{button, column, container, row, stack, text, text_input};
use iced::{alignment, Border, Color, Element, Length};
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
    /// True when we pre-filled an existing saved password
    pub was_prefilled: bool,
    /// True when opened via "Edit" — we label the confirm button "Save" instead of "Connect"
    pub is_edit_mode: bool,
}

/// What the caller should do after calling `try_connect_selected`.
pub enum ConnectAction {
    /// Show the password prompt — secured network with no saved credential.
    ShowPrompt,
    /// Connect directly with this (ssid, password) — no prompt needed.
    ConnectDirectly(String, String),
    /// The selected network is already the active connection.
    AlreadyConnected,
    /// Nothing selected / empty list.
    Nothing,
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
        let scanning = Arc::new(Mutex::new(false));
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

    fn trigger_scan(
        networks: Arc<Mutex<Vec<WifiNetwork>>>,
        last_scan: Arc<Mutex<Instant>>,
        scanning: Arc<Mutex<bool>>,
    ) {
        if *scanning.lock().unwrap() {
            return;
        }
        *scanning.lock().unwrap() = true;
        std::thread::spawn(move || {
            let result = fetch_wifi_networks();
            if let Ok(mut n) = networks.lock() {
                *n = result;
            }
            if let Ok(mut t) = last_scan.lock() {
                *t = Instant::now();
            }
            if let Ok(mut s) = scanning.lock() {
                *s = false;
            }
        });
    }

    pub fn maybe_rescan(&self) {
        let elapsed = self
            .last_scan
            .lock()
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
        if self.connect_prompt.is_some() {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.update_window();
        }
    }

    pub fn arrow_down(&mut self) {
        if self.connect_prompt.is_some() {
            return;
        }
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

    /// Smart connect entry-point — decide what to do without showing a prompt if possible.
    ///
    /// Rules:
    /// 1. Already connected → `AlreadyConnected` (caller should ignore / show info)
    /// 2. Open network       → `ConnectDirectly("ssid", "")`
    /// 3. Secured + saved pw → `ConnectDirectly("ssid", saved_pw)`
    /// 4. Secured + no pw    → `ShowPrompt`
    pub fn try_connect_selected(&self) -> ConnectAction {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        let net = match networks.get(self.selected_index) {
            Some(n) => n,
            None => return ConnectAction::Nothing,
        };

        if net.connected {
            return ConnectAction::AlreadyConnected;
        }

        if !net.secured {
            return ConnectAction::ConnectDirectly(net.ssid.clone(), String::new());
        }

        if let Some(pw) = wifi_credentials::get_password(&net.ssid) {
            eprintln!("[Wifi] Auto-using saved password for '{}'", net.ssid);
            return ConnectAction::ConnectDirectly(net.ssid.clone(), pw);
        }

        ConnectAction::ShowPrompt
    }

    /// Open connect prompt, pre-filling the saved password if available.
    pub fn open_connect_prompt(&mut self) {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        if let Some(net) = networks.get(self.selected_index) {
            let saved = wifi_credentials::get_password(&net.ssid);
            let (password, was_prefilled) = match saved {
                Some(pw) => (pw, true),
                None => (String::new(), false),
            };
            self.connect_prompt = Some(ConnectPrompt {
                ssid: net.ssid.clone(),
                secured: net.secured,
                password,
                input_id: iced::widget::Id::unique(),
                show_password: false,
                was_prefilled,
                is_edit_mode: false,
            });
        }
    }

    /// Open the prompt in "Edit" mode — only for networks with a saved password.
    /// The user can change the stored password without connecting immediately.
    pub fn open_edit_prompt(&mut self) {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        if let Some(net) = networks.get(self.selected_index) {
            // Only allow edit when there's already a saved password
            if let Some(pw) = wifi_credentials::get_password(&net.ssid) {
                self.connect_prompt = Some(ConnectPrompt {
                    ssid: net.ssid.clone(),
                    secured: net.secured,
                    password: pw,
                    input_id: iced::widget::Id::unique(),
                    show_password: true, // show password by default in edit mode
                    was_prefilled: true,
                    is_edit_mode: true,
                });
            }
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

    pub fn toggle_show_password(&mut self) {
        if let Some(ref mut p) = self.connect_prompt {
            p.show_password = !p.show_password;
        }
    }

    /// Returns (ssid, password) and closes the prompt.
    pub fn take_connect_action(&mut self) -> Option<(String, String)> {
        self.connect_prompt.take().map(|p| (p.ssid, p.password))
    }

    /// In edit mode, just save the new password without connecting. Returns the ssid.
    pub fn take_edit_save(&mut self) -> Option<(String, String)> {
        self.connect_prompt.take().and_then(|p| {
            if p.is_edit_mode {
                Some((p.ssid, p.password))
            } else {
                None
            }
        })
    }

    /// Return SSID of the currently selected network, if any.
    pub fn selected_ssid(&self) -> Option<String> {
        self.networks
            .lock()
            .ok()
            .and_then(|n| n.get(self.selected_index).map(|net| net.ssid.clone()))
    }

    /// Return whether the selected network is currently connected.
    pub fn selected_is_connected(&self) -> bool {
        self.networks
            .lock()
            .ok()
            .and_then(|n| n.get(self.selected_index).map(|net| net.connected))
            .unwrap_or(false)
    }

    pub fn is_scanning(&self) -> bool {
        *self.scanning.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Return a snapshot of the network list for auto-connect logic.
    pub fn get_networks_snapshot(&self) -> Vec<WifiNetwork> {
        self.networks.lock().map(|n| n.clone()).unwrap_or_default()
    }

    // ── view dispatch ─────────────────────────────────────────────────────

    pub fn view<'a>(
        &'a self,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        if let Some(ref prompt) = self.connect_prompt {
            return self.view_connect_prompt(prompt, theme, bg_with_alpha, font, font_size);
        }
        self.view_list(theme, bg_with_alpha, font, font_size)
    }

    // ── list view ─────────────────────────────────────────────────────────

    fn view_list<'a>(
        &'a self,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let networks = self.networks.lock().map(|n| n.clone()).unwrap_or_default();
        let scanning = self.is_scanning();
        let rfs = font_size * 0.9;
        let dim = Color::from_rgb(theme.color6.r, theme.color6.g, theme.color6.b);

        let header = container(
            row![
                text("").width(Length::Fixed(30.0)),
                text("NETWORK")
                    .font(font)
                    .size(font_size * 0.68)
                    .color(dim)
                    .width(Length::Fill),
                text("SIG")
                    .font(font)
                    .size(font_size * 0.68)
                    .color(dim)
                    .width(Length::Fixed(24.0)),
            ]
            .spacing(2)
            .align_y(alignment::Vertical::Center),
        )
        .padding(iced::padding::horizontal(8).vertical(2))
        .width(Length::Fill);

        let mut rows = column![].spacing(0);

        if networks.is_empty() {
            let msg = if scanning {
                "Scanning for networks…"
            } else {
                "No networks found"
            };
            rows = rows.push(
                container(text(msg).font(font).size(rfs).color(Color::from_rgb(
                    theme.color6.r,
                    theme.color6.g,
                    theme.color6.b,
                )))
                .padding(iced::padding::horizontal(12).vertical(12))
                .width(Length::Fill),
            );
        } else {
            let window_end = (self.window_start + WINDOW_SIZE).min(networks.len());
            for idx in self.window_start..window_end {
                let net = &networks[idx];
                let selected = idx == self.selected_index;
                let is_saved = wifi_credentials::has_saved(&net.ssid);

                let row_bg: Option<iced::Background> = if selected {
                    Some(Color::from_rgb(theme.color3.r, theme.color3.g, theme.color3.b).into())
                } else if idx % 2 == 0 {
                    Some(Color::from_rgb(theme.color0.r, theme.color0.g, theme.color0.b).into())
                } else {
                    None
                };

                let ssid_color = if selected {
                    theme.foreground
                } else if net.connected {
                    theme.color2
                } else {
                    theme.color6
                };
                let sig_col = sig_color(theme, net);

                // Show a small floppy/save icon if we have a saved credential
                let saved_icon = if is_saved { "󰆓" } else { " " };
                let saved_color = Color::from_rgb(theme.color2.r, theme.color2.g, theme.color2.b);

                let net_row = row![
                    text(if selected { ">>" } else { "  " })
                        .font(font)
                        .size(rfs)
                        .color(ssid_color)
                        .width(Length::Fixed(20.0)),
                    text(if net.connected { "✓" } else { " " })
                        .font(font)
                        .size(rfs)
                        .color(theme.color2)
                        .width(Length::Fixed(14.0)),
                    text(net.ssid.clone())
                        .font(font)
                        .size(rfs)
                        .color(ssid_color)
                        .width(Length::Fill),
                    // saved indicator
                    text(saved_icon)
                        .font(font)
                        .size(rfs * 0.78)
                        .color(saved_color)
                        .width(Length::Fixed(14.0)),
                    text(if net.secured { " 󰌾" } else { " " })
                        .font(font)
                        .size(rfs * 0.85)
                        .color(Color::from_rgb(
                            theme.color6.r,
                            theme.color6.g,
                            theme.color6.b
                        ))
                        .width(Length::Fixed(16.0)),
                    text(signal_icon(net.signal))
                        .font(font)
                        .size(rfs * 1.05)
                        .color(sig_col)
                        .width(Length::Fixed(16.0)),
                ]
                .spacing(2)
                .align_y(alignment::Vertical::Center)
                .width(Length::Fill);

                let sep = container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fixed(1.0))
                    .style(move |_| container::Style {
                        background: Some(
                            Color::from_rgb(theme.color6.r, theme.color6.g, theme.color6.b).into(),
                        ),
                        ..Default::default()
                    });

                rows = rows.push(
                    container(column![net_row, sep].spacing(0))
                        .padding(iced::padding::horizontal(8).vertical(4))
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: row_bg,
                            ..Default::default()
                        }),
                );
            }
        }

        // --- Determine state of selected network for bottom bar ---
        let selected_net = networks.get(self.selected_index);
        let selected_is_saved = selected_net
            .map(|n| wifi_credentials::has_saved(&n.ssid))
            .unwrap_or(false);
        let selected_is_connected = selected_net.map(|n| n.connected).unwrap_or(false);

        // status text
        let pos_txt = if networks.len() > WINDOW_SIZE {
            let end = (self.window_start + WINDOW_SIZE).min(networks.len());
            format!("{}-{}/{}", self.window_start + 1, end, networks.len())
        } else {
            String::new()
        };
        let scan_txt = if scanning { "..." } else { "" };
        let status_str = [scan_txt, pos_txt.as_str()]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");

        // Bottom bar: Back | Scan | [status] | Edit | Forget | Connect/Disconnect
        let forget_btn: Element<'a, Message> = if selected_is_saved || selected_is_connected {
            pill_btn_colored("Forget", theme.color1, theme, font, font_size * 0.88)
                .on_press(Message::WifiForgetNetwork)
                .into()
        } else {
            // placeholder so layout stays consistent
            container(text(""))
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(1.0))
                .into()
        };

        // "Edit" — ONLY shown when there is a saved password for this network
        let edit_btn: Element<'a, Message> = if selected_is_saved {
            pill_btn("Edit", theme, font, font_size * 0.88)
                .on_press(Message::WifiEditNetwork)
                .into()
        } else {
            container(text(""))
                .width(Length::Fixed(1.0))
                .height(Length::Fixed(1.0))
                .into()
        };

        // Right action: "Disconnect" when connected, "Connect" otherwise
        let action_btn: Element<'a, Message> = if selected_is_connected {
            pill_btn_colored("Disconnect", theme.color1, theme, font, font_size * 0.88)
                .on_press(Message::WifiDisconnect)
                .into()
        } else {
            pill_btn("Connect", theme, font, font_size * 0.88)
                .on_press(Message::WifiOpenConnect)
                .into()
        };

        let bottom_bar = container(
            row![
                pill_btn("Back", theme, font, font_size * 0.88).on_press(Message::GoBackToServices),
                pill_btn("Scan", theme, font, font_size * 0.88).on_press(Message::WifiForceScan),
                container(text(status_str).font(font).size(font_size * 0.72).color(
                    Color::from_rgb(theme.color6.r, theme.color6.g, theme.color6.b)
                ))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .align_y(alignment::Vertical::Center),
                edit_btn,
                forget_btn,
                action_btn,
            ]
            .spacing(5)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill),
        )
        .padding(iced::padding::horizontal(8).vertical(6))
        .width(Length::Fill);

        let inner = column![
            header,
            hairline(theme),
            container(rows).width(Length::Fill).height(Length::Fill),
            hairline(theme),
            bottom_bar,
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        panel_frame(inner, theme, bg_with_alpha, font, font_size, " Wifi ")
    }

    // ── connect prompt view ───────────────────────────────────────────────

    fn view_connect_prompt<'a>(
        &'a self,
        prompt: &'a ConnectPrompt,
        theme: &'a Theme,
        bg_with_alpha: Color,
        font: iced::Font,
        font_size: f32,
    ) -> Element<'a, Message> {
        let dim = Color::from_rgb(theme.color6.r, theme.color6.g, theme.color6.b);

        // Title row
        let mode_label = if prompt.is_edit_mode {
            " Edit Password "
        } else {
            " Wifi "
        };
        let ssid_row = container(
            row![
                text(signal_icon(100))
                    .font(font)
                    .size(font_size * 1.1)
                    .color(theme.color2),
                text("  "),
                text(&prompt.ssid)
                    .font(font)
                    .size(font_size * 1.05)
                    .color(theme.color6),
                container(text("")).width(Length::Fill),
                if prompt.secured {
                    text("secured").font(font).size(font_size * 0.78).color(dim)
                } else {
                    text("open")
                        .font(font)
                        .size(font_size * 0.78)
                        .color(theme.color2)
                },
            ]
            .spacing(0)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill),
        )
        .padding(iced::padding::horizontal(12).vertical(12))
        .width(Length::Fill);

        // ── body ──────────────────────────────────────────────────────────
        let body: Element<'a, Message> =
            if prompt.secured {
                // Input style: transparent bg, no inner border
                let input_style = move |_: &_, _: _| text_input::Style {
                    background: iced::Background::Color(Color::TRANSPARENT),
                    border: Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    icon: Color::TRANSPARENT,
                    placeholder: Color::from_rgb(theme.color6.r, theme.color6.g, theme.color6.b),
                    value: theme.color6,
                    selection: theme.color4,
                };

                // Build two complete widget trees — the only reliable way to
                // conditionally apply .password() since it returns a different type.
                // Note: .password() is not available in this Iced version, so we use plain text.
                let pw_field: Element<'a, Message> = if prompt.show_password {
                    // plain text — user sees what they typed
                    text_input("", &prompt.password)
                        .on_input(Message::WifiPasswordInput)
                        .on_submit(Message::WifiDoConnect)
                        .id(prompt.input_id.clone())
                        .font(font)
                        .size(font_size)
                        .padding(iced::padding::horizontal(10).vertical(11))
                        .style(input_style)
                        .into()
                } else {
                    // Default view — plain text (password masking not available in this Iced version)
                    text_input("", &prompt.password)
                        .on_input(Message::WifiPasswordInput)
                        .on_submit(Message::WifiDoConnect)
                        .id(prompt.input_id.clone())
                        .font(font)
                        .size(font_size)
                        .padding(iced::padding::horizontal(10).vertical(11))
                        .style(input_style)
                        .into()
                };

                // Eye button — brighter when showing, dimmer when hidden
                let eye_icon = if prompt.show_password { "hide" } else { "show" };
                let eye_alpha: f32 = 1.0;

                let eye_btn =
                    button(
                        container(text(eye_icon).font(font).size(font_size).color(
                            Color::from_rgb(theme.color6.r, theme.color6.g, theme.color6.b),
                        ))
                        .width(Length::Fixed(40.0))
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                    )
                    .on_press(Message::WifiTogglePasswordVisibility)
                    .padding(0)
                    .style(move |_, status| match status {
                        iced::widget::button::Status::Hovered => button::Style {
                            background: Some(
                                Color::from_rgb(theme.color3.r, theme.color3.g, theme.color3.b)
                                    .into(),
                            ),
                            ..Default::default()
                        },
                        _ => button::Style {
                            background: Some(Color::TRANSPARENT.into()),
                            ..Default::default()
                        },
                    });

                // Thin vertical separator
                let vline = container(text(""))
                    .width(Length::Fixed(1.0))
                    .height(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(
                            Color::from_rgb(theme.color3.r, theme.color3.g, theme.color3.b).into(),
                        ),
                        ..Default::default()
                    });

                // ── Terminal-style password box ────────────────────────────
                //
                //   ┌─ Password ───────────────────────────────────┐
                //   │                                          [eye]│
                //   └──────────────────────────────────────────────┘
                //
                // Outer stack: the border box + floating " Password " label.

                let input_row = row![
                    container(pw_field).width(Length::Fill).height(Length::Fill),
                    vline,
                    eye_btn,
                ]
                .spacing(0)
                .align_y(alignment::Vertical::Center)
                .width(Length::Fill)
                .height(Length::Fill);

                // The box that holds the input — fixed height so it's tall enough
                let pw_box = container(stack![
                    // border container — padding-top leaves room for the label
                    container(
                        container(input_row)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(iced::padding::top(10).bottom(0).horizontal(0))
                            .style(move |_| container::Style {
                                background: None,
                                border: Border {
                                    color: theme.color3,
                                    width: 1.5,
                                    radius: 0.0.into(),
                                },
                                ..Default::default()
                            })
                    )
                    .padding(iced::padding::top(8))
                    .width(Length::Fill)
                    .height(Length::Fill),
                    // floating label cutting into the top border
                    container(
                        container(
                            text(if prompt.is_edit_mode {
                                " New Password "
                            } else {
                                " Password "
                            })
                            .font(font)
                            .size(font_size)
                            .color(theme.color6)
                        )
                        .style(move |_| container::Style {
                            background: Some(bg_with_alpha.into()),
                            ..Default::default()
                        })
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                    )
                    .padding(iced::padding::left(10).top(1))
                    .width(Length::Shrink)
                    .height(Length::Shrink),
                ])
                .width(Length::Fill)
                .height(Length::Fixed(58.0)); // tall enough to see text clearly

                // Hint line
                let hint: Element<'a, Message> = if prompt.is_edit_mode {
                    container(
                        text("Editing saved password — Save to update without connecting")
                            .font(font)
                            .size(font_size * 0.72)
                            .color(Color::from_rgb(
                                theme.color6.r,
                                theme.color6.g,
                                theme.color6.b,
                            )),
                    )
                    .padding(iced::padding::horizontal(12).top(4).bottom(0))
                    .width(Length::Fill)
                    .into()
                } else if prompt.was_prefilled {
                    container(
                        text("Using saved password")
                            .font(font)
                            .size(font_size * 0.78)
                            .color(Color::from_rgb(
                                theme.color6.r,
                                theme.color6.g,
                                theme.color6.b,
                            )),
                    )
                    .padding(iced::padding::horizontal(12).top(4).bottom(0))
                    .width(Length::Fill)
                    .into()
                } else {
                    container(text(""))
                        .width(Length::Fixed(1.0))
                        .height(Length::Fixed(1.0))
                        .into()
                };

                container(
                    container(column![pw_box, hint].spacing(0))
                        .padding(iced::padding::horizontal(12).top(12).bottom(8))
                        .width(Length::Fill),
                )
                .width(Length::Fill)
                .into()
            } else {
                container(
                    text("Open network — no password required")
                        .font(font)
                        .size(font_size * 0.88)
                        .color(dim),
                )
                .padding(iced::padding::horizontal(12).vertical(16))
                .width(Length::Fill)
                .into()
            };

        // Bottom bar — edit mode gets "Save" instead of "Connect"
        let confirm_label = if prompt.is_edit_mode {
            "Save"
        } else {
            "Connect"
        };
        let bottom_bar = container(
            row![
                pill_btn("Cancel", theme, font, font_size * 0.88)
                    .on_press(Message::WifiCloseConnect),
                container(text("")).width(Length::Fill),
                pill_btn(confirm_label, theme, font, font_size * 0.88)
                    .on_press(Message::WifiDoConnect),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center)
            .width(Length::Fill),
        )
        .padding(iced::padding::horizontal(8).vertical(6))
        .width(Length::Fill);

        let inner = column![
            ssid_row,
            hairline(theme),
            container(body).width(Length::Fill).height(Length::Fill),
            hairline(theme),
            bottom_bar,
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

        // Use a different frame title in edit mode
        panel_frame(inner, theme, bg_with_alpha, font, font_size, mode_label)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn hairline<'a>(theme: &'a Theme) -> container::Container<'a, Message> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_| container::Style {
            background: Some(
                Color::from_rgb(theme.color3.r, theme.color3.g, theme.color3.b).into(),
            ),
            ..Default::default()
        })
}

fn pill_btn<'a>(
    label: &'a str,
    theme: &'a Theme,
    font: iced::Font,
    fs: f32,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(text(label).font(font).size(fs).color(theme.color6))
        .padding(iced::padding::horizontal(12).vertical(5))
        .style(move |_, status| match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(
                    Color::from_rgb(theme.color3.r, theme.color3.g, theme.color3.b).into(),
                ),
                border: Border {
                    color: theme.color6,
                    width: 1.5,
                    radius: 0.0.into(),
                },
                ..Default::default()
            },
            _ => iced::widget::button::Style {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: theme.color3,
                    width: 1.5,
                    radius: 0.0.into(),
                },
                ..Default::default()
            },
        })
}

/// Pill button with a custom accent colour (e.g. red for Disconnect / Forget).
fn pill_btn_colored<'a>(
    label: &'a str,
    accent: iced::Color,
    _theme: &'a Theme,
    font: iced::Font,
    fs: f32,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(text(label).font(font).size(fs).color(accent))
        .padding(iced::padding::horizontal(12).vertical(5))
        .style(move |_, status| match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Color::from_rgb(accent.r, accent.g, accent.b).into()),
                border: Border {
                    color: accent,
                    width: 1.5,
                    radius: 0.0.into(),
                },
                ..Default::default()
            },
            _ => iced::widget::button::Style {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: Color::from_rgb(accent.r, accent.g, accent.b),
                    width: 1.5,
                    radius: 0.0.into(),
                },
                ..Default::default()
            },
        })
}

fn sig_color(theme: &Theme, net: &WifiNetwork) -> Color {
    if net.connected {
        return theme.color2;
    }
    match net.signal {
        75..=100 => theme.color2,
        50..=74 => theme.color3,
        25..=49 => theme.color3,
        _ => theme.color1,
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
        container(stack![
            container(
                container(inner.into())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(iced::padding::top(18).left(6).right(6).bottom(6))
                    .style(move |_| container::Style {
                        background: None,
                        border: Border {
                            color: theme.color3,
                            width: 2.0,
                            radius: 0.0.into()
                        },
                        ..Default::default()
                    })
            )
            .padding(iced::padding::top(10))
            .width(Length::Fill)
            .height(Length::Fill),
            container(
                container(text(title).color(theme.color6).font(font).size(font_size))
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
        ])
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

impl Default for WifiPanel {
    fn default() -> Self {
        Self::new()
    }
}
