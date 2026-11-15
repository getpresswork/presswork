use std::sync::LazyLock;

pub use iced::Theme;
pub use iced::theme::Mode;
use iced::theme::Palette;

use crate::color::{Colors, DARK, LIGHT};

static LIGHT_THEME: LazyLock<Theme> = LazyLock::new(|| custom("Presswork Light", &LIGHT));
static DARK_THEME: LazyLock<Theme> = LazyLock::new(|| custom("Presswork Dark", &DARK));

#[must_use]
pub fn light() -> Theme {
    LIGHT_THEME.clone()
}

#[must_use]
pub fn dark() -> Theme {
    DARK_THEME.clone()
}

#[must_use]
pub fn for_mode(mode: Mode) -> Theme {
    match mode {
        Mode::Dark => dark(),
        Mode::Light | Mode::None => light(),
    }
}

#[must_use]
pub fn colors(theme: &Theme) -> &'static Colors {
    if theme.extended_palette().is_dark {
        &DARK
    } else {
        &LIGHT
    }
}

fn custom(name: &'static str, colors: &Colors) -> Theme {
    Theme::custom(
        name,
        Palette {
            background: colors.page,
            text: colors.ink,
            primary: colors.accent,
            success: colors.success,
            warning: colors.warning,
            danger: colors.danger,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_theme_uses_the_page_ink_and_accent_colors() {
        let palette = light().palette();

        assert_eq!(palette.background, LIGHT.page);
        assert_eq!(palette.text, LIGHT.ink);
        assert_eq!(palette.primary, LIGHT.accent);
    }

    #[test]
    fn only_the_dark_theme_is_dark() {
        assert!(dark().extended_palette().is_dark);
        assert!(!light().extended_palette().is_dark);
    }

    #[test]
    fn for_mode_selects_the_theme_of_the_system_mode() {
        assert!(for_mode(Mode::Dark).extended_palette().is_dark);
        assert!(!for_mode(Mode::Light).extended_palette().is_dark);
        assert!(!for_mode(Mode::None).extended_palette().is_dark);
    }

    #[test]
    fn colors_gives_the_tokens_of_the_theme() {
        assert_eq!(colors(&light()), &LIGHT);
        assert_eq!(colors(&dark()), &DARK);
    }
}
