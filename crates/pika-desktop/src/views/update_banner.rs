use iced::widget::{container, row, text};
use iced::{Element, Fill, Theme};

use crate::theme;

/// Full-width banner shown when the app version is below the server minimum.
pub fn view<'a, M: 'a>() -> Element<'a, M, Theme> {
    let row = row![text("A new version of Pika is available. Please update.")
        .color(iced::Color::WHITE)
        .width(Fill)]
    .align_y(iced::Alignment::Center);

    container(row)
        .padding([8, 16])
        .width(Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(theme::accent_blue())),
            ..Default::default()
        })
        .into()
}
