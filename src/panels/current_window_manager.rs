use iced::widget::{container, text, row, button};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;

/// Current Window Manager Panel - appears when hovering at the top of the main window
/// This panel provides quick access to window management features
pub fn current_window_manager_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    current_workspace: usize,
) -> Element<'a, Message> {
    // Show 8 workspaces
    let buttons_row = row![
        workspace_button(theme, font, font_size, 1, current_workspace == 1),
        workspace_button(theme, font, font_size, 2, current_workspace == 2),
        workspace_button(theme, font, font_size, 3, current_workspace == 3),
        workspace_button(theme, font, font_size, 4, current_workspace == 4),
        workspace_button(theme, font, font_size, 5, current_workspace == 5),
        workspace_button(theme, font, font_size, 6, current_workspace == 6),
        workspace_button(theme, font, font_size, 7, current_workspace == 7),
        workspace_button(theme, font, font_size, 8, current_workspace == 8),
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
fn workspace_button<'a>(
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
    workspace_num: usize,
    active: bool,
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
                _ => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.2),
                    theme.color3,
                    1.0,
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

