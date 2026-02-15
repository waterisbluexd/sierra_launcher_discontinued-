use iced::widget::{container, text, column, stack, row, button};
use iced::{Element, Border, Color, Length};
use crate::utils::theme::Theme;
use crate::Message;
use crate::panels::search_bar::SearchBar;
use crate::panels::app_list::AppList;
use crate::panels::clock;
use crate::panels::weather;
use crate::panels::music;
use crate::panels::system;
use crate::panels::services;
use super::mpris_player::MusicPlayer;
use crate::panels::system::system_panel_view;
use crate::panels::wallpaper_panel;
use crate::utils::wallpaper_manager::WallpaperIndex;
use crate::panels::clipboard_panel::clipboard_panel_view;
use crate::Panel;

pub fn right_main_panels_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    search_bar: &'a SearchBar,
    app_list: &'a AppList,
    current_panel: crate::Panel,
    weather_panel: &'a weather::WeatherPanel,
    music_player: &'a MusicPlayer,
    system_panel: &'a system::SystemPanel,
    services_panel: &'a services::ServicesPanel,
    control_center_visible: bool,
    clipboard_visible: bool,
    clipboard_selected_index: usize,
    wallpaper_index: Option<&'a WallpaperIndex>,
    wallpaper_selected_index: usize,
) -> Element<'a, Message> {
    let current_view = match current_panel {
        Panel::Clock => clock::clock_panel_view(theme, bg_with_alpha, font, font_size),
        Panel::Weather => weather_panel.view(theme, bg_with_alpha, font, font_size),
        Panel::Music => music::music_panel_view(theme, bg_with_alpha, font, font_size, music_player),
        Panel::Wallpaper => wallpaper_panel::wallpaper_panel_view(
    theme,
    bg_with_alpha,
    font,
    font_size,
    wallpaper_index,
    wallpaper_selected_index,
),

        Panel::System => system_panel_view(system_panel, theme, bg_with_alpha, font, font_size),
        Panel::Services => services_panel.view(theme, bg_with_alpha, font, font_size),
    };
    
    container(
        stack![
            
            column![
            current_view,
            if !clipboard_visible {
                container(
                    stack![
                        container(
                            container(
                                container(
                                    app_list.view(theme, font, font_size).map(Message::AppListMessage)
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .padding(iced::padding::top(15).right(15).left(15))
                                .style(move |_| container::Style {
                                    background: None,
                                    ..Default::default()
                                }),
                            )
                                .height(Length::Fill)
                                .width(Length::Fill)
                                .style(move |_| container::Style {
                                    background: None,
                                    border: Border {
                                        color: theme.color3,
                                        width: 2.0,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                }),
                        )
                        .padding(iced::padding::top(9))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .style(move |_| container::Style {
                                background: None,
                                ..Default::default()
                            }),

                        container(
                            container( 
                                text(" Apps ")
                                    .color(theme.color6)
                                    .font(font)
                                    .size(font_size)
                            )
                            .width(Length::Shrink)
                            .height(Length::Shrink)
                            .style(move |_| container::Style {
                                background: Some(bg_with_alpha.into()),
                                ..Default::default()
                            }),
                        )
                        .padding(iced::padding::left(8))
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .style(move |_| container::Style {
                            background: None,
                            ..Default::default()
                        }),
                    ]
                )
                .width(Length::Fill)
                .height(Length::FillPortion(2))
                .style(move |_| container::Style {
                    background: None,
                    ..Default::default()
                })
            } else {
                container(text(""))
                    .width(Length::Fill)
                    .height(Length::FillPortion(2))
                    .style(move |_| container::Style {
                        background: None,
                        ..Default::default()
                    })
            },

            if !clipboard_visible {
                container(
                    stack![
                        container(
                            row![
                                container(
                                    search_bar.view(theme, font, font_size).map(Message::SearchBarMessage)
                                )
                                .width(Length::FillPortion(1))
                                .height(Length::Fixed(35.0))
                                .style(move |_| container::Style {
                                    background: Some(bg_with_alpha.into()),
                                    border: Border {
                                        color: theme.color6,
                                        width: 2.0,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                }),

                                container(
                                    button(
                                        container(
                                            text(if control_center_visible { "󰁝" } else { "" })
                                                .color(theme.color6)
                                                .font(font)
                                                .size(font_size * 1.0)
                                                .line_height(0.9)
                                                .center()
                                        )
                                        .width(Length::Fill)
                                        .height(Length::Fill)
                                        .center_x(Length::Fill) 
                                        .center_y(Length::Fill) 
                                    )
                                    .on_press(Message::ToggleControlCenter)
                                    .style(move |_, _| button::Style {
                                        background: Some(Color::TRANSPARENT.into()),
                                        ..Default::default()
                                    }),
                                )
                                .width(Length::Fixed(35.0))
                                .height(Length::Fill)
                                .style(move |_| container::Style {
                                    background: None,
                                    border: Border {
                                        color: theme.color1,
                                        width: 2.0,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                }),
                            ]
                            .spacing(5)
                            .height(Length::Fill)
                        )
                        .padding(iced::padding::top(10))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(move |_| container::Style {
                            background: Some(bg_with_alpha.into()),
                            ..Default::default()
                        }),

                        container(
                            container(
                                text(" Input ")
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
                        .padding(iced::padding::bottom(30).left(8))
                        .width(Length::Fill)
                        .height(Length::Fill)
                    ]
                )
                .width(Length::Fill)
                .height(Length::Fixed(45.0))
                .style(move |_| container::Style {
                    background: Some(bg_with_alpha.into()),
                    ..Default::default()
                })
            } else {
                container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fixed(45.0))
                    .style(move |_| container::Style {
                        background: None,
                        ..Default::default()
                    })
            },

            ].spacing(5),

            if clipboard_visible {
                clipboard_panel_view(theme, bg_with_alpha, font, font_size, clipboard_selected_index)
            } else {
                container(text(""))
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .into()
            },
            
            if control_center_visible {
                container(
                    column![
                        container(
                            button(
                                container(
                                    text("󰤄")
                                        .color(theme.color6)
                                        .font(font)
                                        .size(font_size * 1.3)
                                        .line_height(0.9)
                                        .center()
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x(Length::Fill) 
                                .center_y(Length::Fill) 
                            )
                            .on_press(Message::SleepModeTheSystem)
                            .style(move |_, status| {
                                match status {
                                    iced::widget::button::Status::Hovered => button::Style {
                                        background: Some(bg_with_alpha.into()),
                                        border: Border {
                                            color: theme.color7,
                                            width: 2.0,
                                            radius: 0.0.into(),
                                        },
                                        ..Default::default()
                                    },
                                    _ => button::Style {
                                        background: Some(bg_with_alpha.into()),
                                        border: Border {
                                            color: theme.color1,
                                            width: 2.0,
                                            radius: 0.0.into(),
                                        },
                                        ..Default::default()
                                    }
                                }
                            }),
                        )
                        .width(Length::Fixed(35.0))
                        .height(Length::Fixed(35.0)),

                        container(
                            button(
                                container(
                                    text("󰜉")
                                        .color(theme.color6)
                                        .font(font)
                                        .size(font_size * 1.3)
                                        .line_height(0.9)
                                        .center()
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x(Length::Fill) 
                                .center_y(Length::Fill) 
                            )
                            .on_press(Message::RestartTheSystem)
                            .style(move |_, status| {
                                match status {
                                    iced::widget::button::Status::Hovered => button::Style {
                                        background: Some(bg_with_alpha.into()),
                                        border: Border {
                                            color: theme.color7,
                                            width: 2.0,
                                            radius: 0.0.into(),
                                        },
                                        ..Default::default()
                                    },
                                    _ => button::Style {
                                        background: Some(bg_with_alpha.into()),
                                        border: Border {
                                            color: theme.color1,
                                            width: 2.0,
                                            radius: 0.0.into(),
                                        },
                                        ..Default::default()
                                    }
                                }
                            }),
                        )
                        .width(Length::Fixed(35.0))
                        .height(Length::Fixed(35.0)),

                        container(
                            button(
                                container(
                                    text("")
                                        .color(theme.color6)
                                        .font(font)
                                        .size(font_size * 1.0)
                                        .line_height(0.9)
                                        .center()
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x(Length::Fill) 
                                .center_y(Length::Fill) 
                            )
                            .on_press(Message::PowerOffTheSystem)
                            .style(move |_, status| {
                                match status {
                                    iced::widget::button::Status::Hovered => button::Style {
                                        background: Some(bg_with_alpha.into()),
                                        border: Border {
                                            color: theme.color7,
                                            width: 2.0,
                                            radius: 0.0.into(),
                                        },
                                        ..Default::default()
                                    },
                                    _ => button::Style {
                                        background: Some(bg_with_alpha.into()),
                                        border: Border {
                                            color: theme.color1,
                                            width: 2.0,
                                            radius: 0.0.into(),
                                        },
                                        ..Default::default()
                                    }
                                }
                            }),
                        )
                        .width(Length::Fixed(35.0))
                        .height(Length::Fixed(35.0)),
                    ]
                    .spacing(5)
                )
                .padding(iced::padding::left(386).bottom(40).top(539))
                .width(Length::Fill)
                .height(Length::Fill)
            } else {
                container(text(""))
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .into()
            }
        ]
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::Style {
        background: None,
        ..Default::default()
    })
    .into()
}
