use iced::widget::{text_input, Id};
use iced::{Element, Color, Border};

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Submitted,
}

pub struct SearchBar {
    pub input_value: String,
    pub input_id: Id,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            input_value: String::new(),
            input_id: Id::unique(),
        }
    }

    pub fn view<'a>(&self, theme: &'a crate::utils::theme::Theme, font: iced::Font, font_size: f32) -> Element<'a, Message> {
        text_input(
            "Search for apps...",
            &self.input_value,
        )
        .on_input(Message::InputChanged)
        .id(self.input_id.clone())
        .on_submit(Message::Submitted)
        .size(font_size)
        .font(font)
        .padding(10)
        .style(move |_theme, _status| {
            text_input::Style {
                background: iced::Background::Color(Color::TRANSPARENT),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                icon: Color::TRANSPARENT,
                placeholder: theme.foreground,
                value: theme.foreground,
                selection: theme.color4,
            }
        })
        .into()
    }
}