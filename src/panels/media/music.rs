use super::mpris_player::MusicPlayer;
use crate::utils::theme::Theme;
use crate::Message;
use iced::widget::{button, column, container, image, row, slider, stack, text};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length};

pub fn music_panel_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    music_player: &'a MusicPlayer,
) -> Element<'a, Message> {
    let music_state = &music_player.state;
    let play_pause_icon = if music_state.is_playing { "⏸" } else { "▶" };

    container(
        container(stack![
            container(
                container(if music_state.player_available {
                    row![
                        // Left side: Thumbnail square
                        container(
                            image(image::Handle::from_path(
                                music_state.thumbnail_path.as_deref().unwrap_or("")
                            ))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .content_fit(ContentFit::Cover)
                        )
                        .width(Length::Fixed(180.0))
                        .height(Length::Fixed(180.0))
                        .padding(6)
                        .style(move |_| container::Style {
                            background: None,
                            border: Border {
                                color: theme.color3,
                                width: 2.0,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        }),
                        // Right side: Music info and controls
                        column![
                            // Title
                            container(
                                text(&music_state.song_name)
                                    .color(theme.color1)
                                    .font(font)
                                    .size(font_size * 1.2)
                            )
                            .width(Length::Fill)
                            .padding(iced::padding::bottom(8)),
                            // Artist
                            container(
                                text(&music_state.artist_name)
                                    .color(Color::from_rgb(
                                        theme.color6.r,
                                        theme.color6.g,
                                        theme.color6.b
                                    ))
                                    .font(font)
                                    .size(font_size * 0.75)
                            )
                            .width(Length::Fill)
                            .padding(iced::padding::bottom(25)),
                            // Progress bar with timestamps
                            row![
                                text(MusicPlayer::format_time(music_state.current_time))
                                    .color(theme.color6)
                                    .font(font)
                                    .size(font_size * 0.8),
                                slider(
                                    0.0..=music_state.total_time.max(1.0),
                                    music_state.current_time,
                                    Message::MusicProgressChanged
                                )
                                .width(Length::Fill)
                                .step(1.0)
                                .style(
                                    move |_theme_palette, _status| {
                                        slider::Style {
                                            rail: slider::Rail {
                                                backgrounds: (
                                                    Background::Color(theme.color4),
                                                    Background::Color(Color::from_rgb(
                                                        theme.color6.r,
                                                        theme.color6.g,
                                                        theme.color6.b,
                                                    )),
                                                ),
                                                width: 20.0,
                                                border: Border {
                                                    radius: 0.0.into(),
                                                    ..Default::default()
                                                },
                                            },
                                            handle: slider::Handle {
                                                shape: slider::HandleShape::Rectangle {
                                                    width: 0,
                                                    border_radius: 0.0.into(),
                                                },
                                                background: Background::Color(Color::TRANSPARENT),
                                                border_width: 0.0,
                                                border_color: Color::TRANSPARENT,
                                            },
                                        }
                                    }
                                ),
                                text(MusicPlayer::format_time(music_state.total_time))
                                    .color(theme.color6)
                                    .font(font)
                                    .size(font_size * 0.8),
                            ]
                            .width(Length::Fill)
                            .spacing(10)
                            .align_y(Alignment::Center)
                            .padding(iced::padding::bottom(20)),
                            // Control buttons
                            row![
                                button(
                                    container(
                                        text("⏮").color(theme.color1).font(font).size(font_size)
                                    )
                                    .width(Length::Fixed(50.0))
                                    .height(Length::Fixed(50.0))
                                    .center_x(Length::Shrink)
                                    .center_y(Length::Shrink)
                                )
                                .on_press(Message::MusicPrevious)
                                .style(move |_, _| button::Style {
                                    background: Some(Color::TRANSPARENT.into()),
                                    border: Border {
                                        color: theme.color1,
                                        width: 1.5,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                }),
                                button(
                                    container(
                                        text(play_pause_icon)
                                            .color(theme.color2)
                                            .font(font)
                                            .size(font_size)
                                    )
                                    .width(Length::Fixed(50.0))
                                    .height(Length::Fixed(50.0))
                                    .center_x(Length::Shrink)
                                    .center_y(Length::Shrink)
                                )
                                .on_press(Message::MusicPlayPause)
                                .style(move |_, _| button::Style {
                                    background: Some(Color::TRANSPARENT.into()),
                                    border: Border {
                                        color: theme.color2,
                                        width: 1.5,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                }),
                                button(
                                    container(
                                        text("⏭").color(theme.color1).font(font).size(font_size)
                                    )
                                    .width(Length::Fixed(50.0))
                                    .height(Length::Fixed(50.0))
                                    .center_x(Length::Shrink)
                                    .center_y(Length::Shrink)
                                )
                                .on_press(Message::MusicNext)
                                .style(move |_, _| button::Style {
                                    background: Some(Color::TRANSPARENT.into()),
                                    border: Border {
                                        color: theme.color1,
                                        width: 1.5,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                }),
                            ]
                            .spacing(12)
                            .align_y(Alignment::Center)
                        ]
                        .width(Length::Fill)
                        .padding(iced::padding::left(20).right(15))
                        .align_x(Alignment::Start)
                    ]
                    .spacing(0)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .padding(iced::padding::all(20))
                } else {
                    row![column![
                        container(text("")).height(Length::Fill),
                        container(
                            text("No Music Playing")
                                .color(theme.color6)
                                .font(font)
                                .size(font_size * 1.0)
                        )
                        .width(Length::Fill)
                        .center_x(Length::Fill)
                        .padding(iced::padding::bottom(10)),
                        container(
                            text("Start playing music in Spotify, YouTube,")
                                .color(Color::from_rgb(
                                    theme.color6.r,
                                    theme.color6.g,
                                    theme.color6.b
                                ))
                                .font(font)
                                .size(font_size * 0.7)
                        )
                        .width(Length::Fill)
                        .center_x(Length::Fill)
                        .padding(iced::padding::bottom(5)),
                        container(
                            text("or any MPRIS-compatible player")
                                .color(Color::from_rgb(
                                    theme.color6.r,
                                    theme.color6.g,
                                    theme.color6.b
                                ))
                                .font(font)
                                .size(font_size * 0.7)
                        )
                        .width(Length::Fill)
                        .center_x(Length::Fill),
                        container(text("")).height(Length::Fill),
                    ]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)]
                })
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
                    text(" Music ")
                        .color(theme.color6)
                        .font(font)
                        .size(font_size)
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
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: None,
            ..Default::default()
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::Style {
        background: None,
        ..Default::default()
    })
    .into()
}
