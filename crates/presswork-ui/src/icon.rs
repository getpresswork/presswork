use iced::widget::text::LineHeight;
use iced::widget::{Text, text};
pub use lucide_icons::Icon;

use crate::{font, size};

#[must_use]
pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon.unicode().to_string())
        .font(font::ICONS)
        .size(size::ICON)
        .line_height(LineHeight::Relative(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced_test::simulator;

    #[test]
    fn icon_shows_the_code_point_of_the_icon() {
        let mut ui = simulator::<(), _, _>(icon(Icon::Search));

        assert!(ui.find(Icon::Search.unicode().to_string()).is_ok());
    }
}
