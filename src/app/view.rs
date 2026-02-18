use iced::widget::{container, text, stack, row, column};
use iced::{Element, Border, Color, Length};

use crate::app::state::Launcher;
use crate::app::message::Message;
use crate::panels::right_main_panels::right_main_panels_view;

pub fn view(launcher: &Launcher) -> Element<'_, Message> {
    let bg = launcher.theme.background;
    let bg_with_alpha = Color::from_rgb(bg.r, bg.g, bg.b);

    let font = launcher.config.get_font();
    let font_size = launcher.config.font_size.unwrap_or(22.0);

    let title_text = &launcher.config.title_text;
    let total_chars = title_text.chars().count();
    let mut title_column = column![].spacing(0);

    for (i, ch) in title_text.chars().enumerate() {
        let char_color = launcher
            .title_animator
            .get_color_for_char(&launcher.theme, i, total_chars);

        title_column = title_column.push(
            text(ch.to_string())
                .font(font)
                .size(font_size)
                .color(char_color),
        );
    }

    container(
        stack![
            container(
                container(text(""))
                    .padding(9)
                    .height(Length::Fill)
                    .width(Length::Shrink)
                    .style(move |_| container::Style {
                        background: Some(bg_with_alpha.into()),
                        border: Border {
                            color: launcher.theme.color6,
                            width: 2.0,
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    }),
            )
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg_with_alpha.into()),
                ..Default::default()
            }),
            container(
                row![
                    container(text(""))
                        .height(Length::Fill)
                        .width(Length::Shrink),
                    container(right_main_panels_view(
                        &launcher.theme,
                        bg_with_alpha,
                        font,
                        font_size,
                        &launcher.search_bar,
                        &launcher.app_list,
                        launcher.current_panel,
                        &launcher.weather_panel,
                        &launcher.music_player,
                        &launcher.system_panel,
                        &launcher.services_panel,
                        launcher.control_center_visible,
                        launcher.clipboard_visible,
                        launcher.clipboard_selected_index,
                        launcher.wallpaper_index.as_ref(),
                        launcher.wallpaper_selected_index,
                    ))
                    .height(Length::Fill)
                    .width(Length::Fill),
                ]
                .spacing(45),
            )
            .padding(iced::padding::bottom(14).right(14))
            .width(Length::Fill)
            .height(Length::Fill),
            container(
                container(
                    container(title_column)
                        .padding(0)
                        .style(move |_| container::Style {
                            background: Some(bg_with_alpha.into()),
                            ..Default::default()
                        }),
                )
                .padding([20, 10]),
            )
            .width(Length::Fill)
            .height(Length::Fill),
        ],
    )
    .padding(2)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::Style {
        background: Some(bg_with_alpha.into()),
        border: Border {
            color: launcher.theme.border,
            width: 2.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

pub fn popup_view(launcher: &Launcher) -> Element<'_, Message> {
    let bg = launcher.theme.background;
    let bg_with_alpha = Color::from_rgb(bg.r, bg.g, bg.b);

    container(
        text("Popup Panel")
            .size(20)
    )
    .padding(10)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::Style {
        background: Some(bg_with_alpha.into()),
        border: Border {
            color: launcher.theme.border,
            width: 2.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}
