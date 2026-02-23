use iced::widget::{container, text, row, button};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use std::collections::HashSet;

/// Current Window Manager Panel - appears when hovering at the top of the main window
/// This panel provides quick access to window management features
pub fn current_window_manager_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    current_workspace: usize,
) -> Element<'a, Message> {
    // Get workspaces that have windows
    let workspaces_with_windows = get_workspaces_with_windows();
    
    // Show 10 workspaces
    let buttons_row = row![
        workspace_button(theme, font, font_size, 1, current_workspace == 1, workspaces_with_windows.contains(&1)),
        workspace_button(theme, font, font_size, 2, current_workspace == 2, workspaces_with_windows.contains(&2)),
        workspace_button(theme, font, font_size, 3, current_workspace == 3, workspaces_with_windows.contains(&3)),
        workspace_button(theme, font, font_size, 4, current_workspace == 4, workspaces_with_windows.contains(&4)),
        workspace_button(theme, font, font_size, 5, current_workspace == 5, workspaces_with_windows.contains(&5)),
        workspace_button(theme, font, font_size, 6, current_workspace == 6, workspaces_with_windows.contains(&6)),
        workspace_button(theme, font, font_size, 7, current_workspace == 7, workspaces_with_windows.contains(&7)),
        workspace_button(theme, font, font_size, 8, current_workspace == 8, workspaces_with_windows.contains(&8)),
        workspace_button(theme, font, font_size, 9, current_workspace == 9, workspaces_with_windows.contains(&9)),
        workspace_button(theme, font, font_size, 10, current_workspace == 10, workspaces_with_windows.contains(&10)),
        workspace_button(theme, font, font_size, 11, current_workspace == 11, workspaces_with_windows.contains(&11)),
    ]
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

/// Create a workspace button - expanded with label if active, small box otherwise
/// has_windows indicates if there are windows open on this workspace
fn workspace_button<'a>(
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
    workspace_num: usize,
    active: bool,
    has_windows: bool,
) -> Element<'a, Message> {
    if active {
        // Active workspace: expanded with "Workspace - N" label
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
            let (bg_color, border_color, border_width) = match status {
                button::Status::Hovered => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.7),
                    theme.color6,
                    2.0,
                ),
                button::Status::Pressed => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.8),
                    theme.color6,
                    2.0,
                ),
                _ => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.5),
                    theme.color6,
                    2.0,
                ),
            };
            
            button::Style {
                background: Some(bg_color.into()),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    } else {
        // Inactive workspace: small box without label
        // If has_windows, show brighter color to indicate windows are open
        button(
            text("")  // Empty button
        )
        .width(Length::Fixed(30.0))
        .height(Length::Fixed(22.0))
        .on_press(Message::SwitchWorkspace(workspace_num))
        .style(move |_, status| {
            let (bg_color, border_color, border_width) = match status {
                button::Status::Hovered => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.4),
                    theme.color6,
                    1.5,
                ),
                button::Status::Pressed => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.6),
                    theme.color6,
                    2.0,
                ),
                _ => {
                    if has_windows {
                        // Workspace has windows - brighter/highlighted
                        (
                            Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.45),
                            theme.color6,
                            1.5,
                        )
                    } else {
                        // Empty workspace - dimmer
                        (
                            Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.2),
                            theme.color3,
                            1.0,
                        )
                    }
                }
            };
            
            button::Style {
                background: Some(bg_color.into()),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }
}

/// Get the current workspace from Hyprland/Sway
pub fn get_current_workspace() -> usize {
    // Try Hyprland first
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse JSON to get workspace id
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
                    return id as usize;
                }
            }
        }
    }
    
    // Try Sway
    if let Ok(output) = std::process::Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse JSON array to find focused workspace
            if let Ok(workspaces) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(arr) = workspaces.as_array() {
                    for ws in arr {
                        if let Some(focused) = ws.get("focused").and_then(|v| v.as_bool()) {
                            if focused {
                                if let Some(num) = ws.get("num").and_then(|v| v.as_i64()) {
                                    return num as usize;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Default to workspace 1
    1
}

/// Get set of workspace numbers that have windows open
pub fn get_workspaces_with_windows() -> HashSet<usize> {
    let mut result = HashSet::new();
    
    // Try Hyprland first
    if let Ok(output) = std::process::Command::new("hyprctl")
        .args(["workspaces", "-j"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse JSON array of workspaces
            if let Ok(workspaces) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(arr) = workspaces.as_array() {
                    for ws in arr {
                        // Get workspace id and check if it has windows
                        if let Some(id) = ws.get("id").and_then(|v| v.as_u64()) {
                            // Check if workspace has windows (windows count > 0)
                            let windows_count = ws.get("windows")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            if windows_count > 0 {
                                result.insert(id as usize);
                            }
                        }
                    }
                }
            }
            return result;
        }
    }
    
    // Try Sway
    if let Ok(output) = std::process::Command::new("swaymsg")
        .args(["-t", "get_workspaces"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse JSON array of workspaces
            if let Ok(workspaces) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(arr) = workspaces.as_array() {
                    for ws in arr {
                        // Get workspace num and check if it has windows
                        if let Some(num) = ws.get("num").and_then(|v| v.as_i64()) {
                            // Check if workspace has windows (urgent or visible typically means windows)
                            let focused = ws.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
                            let visible = ws.get("visible").and_then(|v| v.as_bool()).unwrap_or(false);
                            // In Sway, if visible or focused, it has windows
                            if focused || visible {
                                result.insert(num as usize);
                            }
                        }
                    }
                }
            }
        }
    }
    
    result
}

