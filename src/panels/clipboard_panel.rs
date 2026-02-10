use iced::widget::{container, text, stack, column, scrollable, row};
use iced::{Element, Border, Color, Length};
use crate::utils::theme::Theme;
use crate::Message;

const PREVIEW_LINES: usize = 3;
const CHARS_PER_LINE: usize = 40;
const WINDOW_SIZE: usize = 7;

/// Build preview lines safely (UTF-8 safe, owned Strings)
fn create_preview_lines(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut words = content.split_whitespace().peekable();

    while let Some(word) = words.next() {
        let extra = if current.is_empty() { word.len() } else { word.len() + 1 };

        if current.len() + extra <= CHARS_PER_LINE {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        } else {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }

            if word.len() > CHARS_PER_LINE {
                // FIX: UTF-8 safe truncation
                let mut truncated = String::new();
                let mut char_count = 0;
                for ch in word.chars() {
                    if char_count >= CHARS_PER_LINE.saturating_sub(3) {
                        break;
                    }
                    truncated.push(ch);
                    char_count += 1;
                }
                truncated.push_str("...");
                lines.push(truncated);
            } else {
                current.push_str(word);
            }
        }

        if lines.len() >= PREVIEW_LINES {
            break;
        }
    }

    if !current.is_empty() && lines.len() < PREVIEW_LINES {
        lines.push(current);
    }

    if words.peek().is_some() && !lines.is_empty() {
        if let Some(last) = lines.last_mut() {
            // FIX: UTF-8 safe truncation for ellipsis
            if last.chars().count() > CHARS_PER_LINE.saturating_sub(3) {
                let mut truncated = String::new();
                let mut char_count = 0;
                for ch in last.chars() {
                    if char_count >= CHARS_PER_LINE.saturating_sub(3) {
                        break;
                    }
                    truncated.push(ch);
                    char_count += 1;
                }
                *last = truncated;
            }
            last.push_str("...");
        }
    }

    if lines.is_empty() {
        lines.push("(empty)".to_string());
    }

    lines
}

pub fn clipboard_panel_view<'a>(
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
    selected_index: usize,
) -> Element<'a, Message> {
    let items = crate::utils::data::search_items("");
    let mut list = column![].spacing(1);

    if items.is_empty() {
        list = list.push(
            container(
                column![
                    text("No clipboard history yet")
                        .font(font)
                        .size(font_size)
                        .color(theme.color6),
                    text(""),
                    text("Copy something to get started!")
                        .font(font)
                        .size(font_size * 0.8)
                        .color(Color::from_rgba(
                            theme.color6.r,
                            theme.color6.g,
                            theme.color6.b,
                            0.5
                        )),
                ]
                .spacing(4),
            )
            .padding(20)
            .width(Length::Fill)
            .center_x(Length::Fill),
        );
    } else {
        let half = WINDOW_SIZE / 2;
        let window_start = selected_index
            .saturating_sub(half)
            .min(items.len().saturating_sub(WINDOW_SIZE));
        let window_end = (window_start + WINDOW_SIZE).min(items.len());

        list = list.push(container(text("")).height(Length::Fixed(8.0)));

        for idx in window_start..window_end {
            let item = &items[idx];
            let content = item.full_content();
            let preview_lines = create_preview_lines(&content);

            let selected = idx == selected_index;
            let fg = if selected { theme.background } else { theme.foreground };
            let number_color = if selected { theme.background } else { theme.color3 };

            let bg = if selected {
                Some(theme.color3.into())
            } else if idx % 2 == 0 {
                Some(Color::from_rgba(
                    theme.color0.r,
                    theme.color0.g,
                    theme.color0.b,
                    0.1,
                ).into())
            } else {
                None
            };

            let mut item_column = column![].spacing(2);

            if let Some(first) = preview_lines.first() {
                item_column = item_column.push(
                    row![
                        text(if selected { ">>" } else { "  " })
                            .font(font)
                            .size(font_size * 0.8)
                            .color(fg),
                        text(format!("{}. ", idx + 1))
                            .font(font)
                            .size(font_size * 0.8)
                            .color(number_color),
                        text(first.clone())
                            .font(font)
                            .size(font_size * 0.8)
                            .color(fg),
                    ]
                    .spacing(4),
                );
            }

            for line in preview_lines.iter().skip(1) {
                item_column = item_column.push(
                    row![
                        text("      ").font(font).size(font_size * 0.8),
                        text(line.clone())
                            .font(font)
                            .size(font_size * 0.8)
                            .color(fg),
                    ]
                    .spacing(4),
                );
            }

            item_column = item_column.push(
                container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fixed(1.0))
                    .style(move |_| container::Style {
                        background: Some(Color::from_rgba(
                            theme.color6.r,
                            theme.color6.g,
                            theme.color6.b,
                            0.2,
                        ).into()),
                        ..Default::default()
                    }),
            );

            list = list.push(
                container(item_column)
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: bg,
                        border: Border::default(),
                        ..Default::default()
                    }),
            );
        }
    }

    container(
        stack![container(
            container(
                scrollable(list)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .padding(iced::padding::top(7).right(15).left(15).bottom(7))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                border: Border {
                    color: theme.color3,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }),
        ).padding(iced::padding::top(10))
        .width(Length::Fill)
        .height(Length::Fill),
        
        container(
            container(
                text(" Clipboard ")
                .font(font)
                .size(font_size)
                .color(theme.color6),
                )
                .style(move |_| container::Style {
                    background: Some(bg_with_alpha.into()),
                    ..Default::default()
            }),
        )
        .padding(iced::padding::left(8))
        .width(Length::Fill)
        .height(Length::Fill),
        ],
    )
    .padding(iced::padding::top(218))
    .width(Length::Fill)
    .height(Length::FillPortion(1))
    .into()
}