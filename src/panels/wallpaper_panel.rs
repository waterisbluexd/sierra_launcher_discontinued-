use iced::widget::{button, container, image, row, stack, text};
use iced::{Border, Color, ContentFit, Element, Length};

use crate::utils::theme::Theme;
use crate::utils::wallpaper_manager::WallpaperIndex;
use crate::Message;

pub fn wallpaper_panel_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    wallpapers: Option<&'a WallpaperIndex>,
    selected: usize,
) -> Element<'a, Message> {
    let wallpaper_view: Element<'a, Message> = if let Some(index) = wallpapers {
        if let Some(entry) = index.wallpapers.get(selected) {
            let thumb_path = &entry.thumbnail;

            image(image::Handle::from_path(thumb_path))
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Cover)
                .into()
        } else {
            container(
                text("No wallpaper")
                    .font(font)
                    .size(font_size)
                    .color(theme.color6),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        }
    } else {
        container(
            text("No wallpapers found")
                .font(font)
                .size(font_size)
                .color(theme.color6),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    };

    let controls = row![
        container(
            button(
                container(
                    text("◀")
                        .font(font)
                        .size(font_size * 1.6)
                        .color(theme.color6)
                )
                .width(Length::Shrink)
                .height(Length::Shrink)
                .center_x(Length::Shrink)
                .center_y(Length::Shrink)
            )
            .on_press(Message::PrevWallpaper)
            .style(move |_, _| button::Style {
                background: Some(Color::from_rgb(0.0, 0.0, 0.0).into()),
                border: Border {
                    color: theme.color4,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }),
        )
        .width(Length::Shrink)
        .height(Length::Shrink)
        .center_x(Length::Shrink)
        .center_y(Length::Fill)
        .padding(10),
        container(text(""))
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        container(text(""))
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        container(
            button(
                container(
                    text("▶")
                        .font(font)
                        .size(font_size * 1.6)
                        .color(theme.color6)
                )
                .width(Length::Shrink)
                .height(Length::Shrink)
                .center_x(Length::Shrink)
                .center_y(Length::Shrink)
            )
            .on_press(Message::NextWallpaper)
            .style(move |_, _| button::Style {
                background: Some(Color::from_rgb(0.0, 0.0, 0.0).into()),
                border: Border {
                    color: theme.color4,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }),
        )
        .width(Length::Shrink)
        .height(Length::Shrink)
        .center_x(Length::Shrink)
        .center_y(Length::Fill)
        .padding(10),
    ]
    .height(Length::Fill);

    let content = stack![
        container(wallpaper_view)
            .width(Length::Fill)
            .height(Length::Fill),
        container(controls).width(Length::Fill).height(Length::Fill),
    ];

    container(
        container(stack![
            container(
                container(content)
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill)
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
            container(
                container(
                    text(" Wallpapers ")
                        .color(theme.color6)
                        .font(font)
                        .size(font_size),
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
        ])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(1))
    .into()
}
