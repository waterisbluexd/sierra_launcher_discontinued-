use iced::widget::{container, text, row, column, button, Space};
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
) -> Element<'a, Message> {
    let title = text("Window Manager")
        .color(theme.color6)
        .font(font)
        .size(font_size * 1.2);

    let content = column![
        title,
        Space::new().height(Length::Fixed(10.0)),
        row![
            workspace_button(theme, font, font_size, "1", true),
            workspace_button(theme, font, font_size, "2", false),
            workspace_button(theme, font, font_size, "3", false),
            workspace_button(theme, font, font_size, "4", false),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    ]
    .spacing(10)
    .align_x(alignment::Horizontal::Center)
    .width(Length::Fill);

    container(content)
        .padding(15)
        .width(Length::Fill)
        .height(Length::Fill)
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

/// Create a workspace button
fn workspace_button<'a>(
    theme: &'a Theme,
    font: iced::Font,
    font_size: f32,
    label: &'a str,
    active: bool,
) -> Element<'a, Message> {
    let bg_color = if active {
        Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.5)
    } else {
        Color::from_rgba(theme.color3.r, theme.color3.g, theme.color3.b, 0.2)
    };

    let btn = button(
        text(label)
            .color(theme.color6)
            .font(font)
            .size(font_size)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center),
    )
    .padding(8)
    .width(Length::Fixed(40.0))
    .height(Length::Fixed(40.0))
    .style(move |_, _| button::Style {
        background: Some(bg_color.into()),
        border: Border {
            color: if active { theme.color6 } else { theme.color3 },
            width: if active { 2.0 } else { 1.0 },
            radius: 4.0.into(),
        },
        ..Default::default()
    });

    btn.into()
}
