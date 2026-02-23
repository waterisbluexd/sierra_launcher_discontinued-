use iced::widget::{container, text, row, button};
use iced::{Element, Border, Color, Length, alignment};
use crate::utils::theme::Theme;
use crate::Message;
use super::current_window_manager::get_workspaces_with_windows;

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
        button(
            text(format!(" Workspace - {} ", workspace_num))
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
                button::Status::Hovered  => Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.7),
                button::Status::Pressed  => Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.8),
                _ => Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.5),
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
                    theme.color6, 1.5_f32,
                ),
                button::Status::Pressed => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.6),
                    theme.color6, 2.0_f32,
                ),
                _ if has_windows => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.45),
                    theme.color6, 1.5_f32,
                ),
                _ => (
                    Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.2),
                    theme.color3, 1.0_f32,
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
