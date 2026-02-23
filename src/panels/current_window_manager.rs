use iced::widget::{container, text, row, button};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use std::collections::HashSet;
use std::sync::OnceLock;

// ── Compositor detection ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compositor {
    Hyprland,
    Sway,
    Unknown,
}

fn detect_compositor() -> Compositor {
    static COMPOSITOR: OnceLock<Compositor> = OnceLock::new();
    *COMPOSITOR.get_or_init(|| {
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
            eprintln!("[WM] Detected compositor: Hyprland");
            Compositor::Hyprland
        } else if std::env::var("SWAYSOCK").is_ok() {
            eprintln!("[WM] Detected compositor: Sway");
            Compositor::Sway
        } else {
            eprintln!("[WM] Unknown compositor, workspace features disabled");
            Compositor::Unknown
        }
    })
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Returns the currently focused workspace number (1-based).
pub fn get_current_workspace() -> usize {
    match detect_compositor() {
        Compositor::Hyprland => hyprland_current_workspace(),
        Compositor::Sway     => sway_current_workspace(),
        Compositor::Unknown  => 1,
    }
}

/// Returns the set of workspace numbers that have at least one window.
pub fn get_workspaces_with_windows() -> HashSet<usize> {
    match detect_compositor() {
        Compositor::Hyprland => hyprland_workspaces_with_windows(),
        Compositor::Sway     => sway_workspaces_with_windows(),
        Compositor::Unknown  => HashSet::new(),
    }
}

/// Switch to a workspace by number. Fire-and-forget (spawns a thread).
pub fn switch_workspace(num: usize) {
    match detect_compositor() {
        Compositor::Hyprland => {
            std::thread::spawn(move || {
                let out = std::process::Command::new("hyprctl")
                    .args(["dispatch", "workspace", &num.to_string()])
                    .output();
                match out {
                    Ok(o) if o.status.success() =>
                        eprintln!("[WM] Hyprland: switched to workspace {}", num),
                    Ok(o) =>
                        eprintln!("[WM] Hyprland: hyprctl error: {}",
                            String::from_utf8_lossy(&o.stderr)),
                    Err(e) =>
                        eprintln!("[WM] Hyprland: failed to spawn hyprctl: {}", e),
                }
            });
        }
        Compositor::Sway => {
            std::thread::spawn(move || {
                let out = std::process::Command::new("swaymsg")
                    .args(["workspace", &num.to_string()])
                    .output();
                match out {
                    Ok(o) if o.status.success() =>
                        eprintln!("[WM] Sway: switched to workspace {}", num),
                    Ok(o) =>
                        eprintln!("[WM] Sway: swaymsg error: {}",
                            String::from_utf8_lossy(&o.stderr)),
                    Err(e) =>
                        eprintln!("[WM] Sway: failed to spawn swaymsg: {}", e),
                }
            });
        }
        Compositor::Unknown => {
            eprintln!("[WM] Cannot switch workspace: unknown compositor");
        }
    }
}

// ── Hyprland backend ──────────────────────────────────────────────────────────

fn hyprland_current_workspace() -> usize {
    let output = match std::process::Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!("[WM] hyprctl activeworkspace failed: {}",
                String::from_utf8_lossy(&o.stderr));
            return 1;
        }
        Err(e) => {
            eprintln!("[WM] hyprctl spawn failed: {}", e);
            return 1;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(&stdout)
        .ok()
        .and_then(|j| j.get("id")?.as_u64())
        .map(|id| id as usize)
        .unwrap_or(1)
}

fn hyprland_workspaces_with_windows() -> HashSet<usize> {
    let mut result = HashSet::new();

    let output = match std::process::Command::new("hyprctl")
        .args(["workspaces", "-j"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return result,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(serde_json::Value::Array(arr)) =
        serde_json::from_str::<serde_json::Value>(&stdout)
    {
        for ws in arr {
            let id = match ws.get("id").and_then(|v| v.as_u64()) {
                Some(id) => id as usize,
                None => continue,
            };
            let windows = ws.get("windows")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if windows > 0 {
                result.insert(id);
            }
        }
    }

    result
}

// ── Sway backend ──────────────────────────────────────────────────────────────

fn sway_current_workspace() -> usize {
    let output = match std::process::Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            eprintln!("[WM] swaymsg get_workspaces failed: {}",
                String::from_utf8_lossy(&o.stderr));
            return 1;
        }
        Err(e) => {
            eprintln!("[WM] swaymsg spawn failed: {}", e);
            return 1;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(serde_json::Value::Array(arr)) =
        serde_json::from_str::<serde_json::Value>(&stdout)
    {
        for ws in &arr {
            let focused = ws.get("focused")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if focused {
                if let Some(num) = ws.get("num").and_then(|v| v.as_i64()) {
                    return num as usize;
                }
            }
        }
    }

    1
}

fn sway_workspaces_with_windows() -> HashSet<usize> {
    let mut result = HashSet::new();

    let output = match std::process::Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return result,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(serde_json::Value::Array(arr)) =
        serde_json::from_str::<serde_json::Value>(&stdout)
    {
        for ws in arr {
            let num = match ws.get("num").and_then(|v| v.as_i64()) {
                Some(n) if n > 0 => n as usize,
                _ => continue,
            };

            // Sway workspace JSON has a "representation" field that is null
            // when the workspace is empty and a string like "H[term firefox]"
            // when it has windows. That is the most reliable empty-check.
            // As a second signal we also count "nodes" and "floating_nodes".
            let has_windows = {
                let repr_non_null = ws.get("representation")
                    .map(|v| !v.is_null())
                    .unwrap_or(false);

                let node_count = ws.get("nodes")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                let float_count = ws.get("floating_nodes")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                repr_non_null || node_count > 0 || float_count > 0
            };

            if has_windows {
                result.insert(num);
            }
        }
    }

    result
}

// ── View ──────────────────────────────────────────────────────────────────────

/// Workspace switcher bar — appears in the popup at the top of the window.
pub fn current_window_manager_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    current_workspace: usize,
) -> Element<'a, Message> {
    let workspaces_with_windows = get_workspaces_with_windows();

    let buttons_row = row(
        (1..=11)
            .map(|n| workspace_button(
                theme, font, font_size, n,
                current_workspace == n,
                workspaces_with_windows.contains(&n),
            ))
            .collect::<Vec<_>>(),
    )
    .spacing(4)
    .align_y(alignment::Vertical::Center)
    .width(Length::Shrink);

    container(buttons_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(iced::padding::horizontal(8))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| container::Style {
            background: Some(bg_with_alpha.into()),
            border: Border {
                color: theme.border,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn workspace_button<'a>(
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
    workspace_num: usize,
    active: bool,
    has_windows: bool,
) -> Element<'a, Message> {
    if active {
        let label = format!(" Workspace - {} ", workspace_num);
        button(
            text(label)
                .color(theme.color6)
                .font(font)
                .size(font_size * 0.85)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center),
        )
        .padding(iced::padding::horizontal(4))
        .height(Length::Fixed(22.0))
        .on_press(Message::SwitchWorkspace(workspace_num))
        .style(move |_, status| {
            let bg = match status {
                button::Status::Hovered  =>
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.7),
                button::Status::Pressed  =>
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.8),
                _ =>
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.5),
            };
            button::Style {
                background: Some(bg.into()),
                border: Border { color: theme.color6, width: 2.0, radius: 0.0.into() },
                ..Default::default()
            }
        })
        .into()
    } else {
        button(text(""))
        .width(Length::Fixed(30.0))
        .height(Length::Fixed(22.0))
        .on_press(Message::SwitchWorkspace(workspace_num))
        .style(move |_, status| {
            let (bg, border_color, border_width) = match status {
                button::Status::Hovered => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.4),
                    theme.color6,
                    1.5_f32,
                ),
                button::Status::Pressed => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.6),
                    theme.color6,
                    2.0_f32,
                ),
                _ if has_windows => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.45),
                    theme.color6,
                    1.5_f32,
                ),
                _ => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.2),
                    theme.color3,
                    1.0_f32,
                ),
            };
            button::Style {
                background: Some(bg.into()),
                border: Border { color: border_color, width: border_width, radius: 0.0.into() },
                ..Default::default()
            }
        })
        .into()
    }
}
