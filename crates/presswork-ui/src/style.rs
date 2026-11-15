use iced::widget::{button, container, rule, text, text_input};
use iced::{Background, Border, Color, Shadow, Theme};

use crate::color::{Colors, mix};
use crate::radius;
use crate::theme::colors;

const BADGE_TINT: f32 = 0.14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Accent,
    Danger,
    Warning,
    Success,
    Info,
}

impl Tone {
    fn color(self, colors: &Colors) -> Color {
        match self {
            Self::Neutral => colors.ink_muted,
            Self::Accent => colors.accent,
            Self::Danger => colors.danger,
            Self::Warning => colors.warning,
            Self::Success => colors.success,
            Self::Info => colors.info,
        }
    }
}

#[must_use]
pub fn page(theme: &Theme) -> container::Style {
    let colors = colors(theme);
    surface(colors.page, colors)
}

#[must_use]
pub fn chrome(theme: &Theme) -> container::Style {
    let colors = colors(theme);
    surface(colors.chrome, colors)
}

#[must_use]
pub fn mark(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(colors(theme).accent.into()),
        border: Border {
            radius: radius::SMALL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn badge(tone: Tone) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let colors = colors(theme);
        let color = tone.color(colors);
        container::Style {
            text_color: Some(color),
            background: Some(mix(colors.chrome, color, BADGE_TINT).into()),
            border: Border {
                radius: radius::SMALL.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }
    }
}

#[must_use]
pub fn hairline(theme: &Theme) -> rule::Style {
    rule::Style {
        color: colors(theme).rule,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

#[must_use]
pub fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(colors(theme).ink_muted),
    }
}

pub fn tone(tone: Tone) -> impl Fn(&Theme) -> text::Style {
    move |theme| text::Style {
        color: Some(tone.color(colors(theme))),
    }
}

pub fn tab(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let colors = colors(theme);
        let (background, text_color) = match (selected, status) {
            (true, _) => (Some(colors.accent_soft), colors.accent),
            (false, button::Status::Hovered | button::Status::Pressed) => {
                (Some(colors.hover), colors.ink)
            }
            (false, button::Status::Disabled) => (None, colors.ink_faint),
            (false, button::Status::Active) => (None, colors.ink_muted),
        };
        button::Style {
            background: background.map(Background::Color),
            text_color,
            border: Border {
                radius: radius::MEDIUM.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

#[must_use]
pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    tab(false)(theme, status)
}

#[must_use]
pub fn field(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let colors = colors(theme);
    let border = match status {
        text_input::Status::Focused { .. } => colors.accent,
        text_input::Status::Hovered => colors.rule_strong,
        text_input::Status::Active | text_input::Status::Disabled => colors.rule,
    };
    text_input::Style {
        background: colors.raised.into(),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::MEDIUM.into(),
        },
        icon: colors.ink_muted,
        placeholder: colors.ink_muted,
        value: colors.ink,
        selection: colors.accent_soft,
    }
}

fn surface(background: Color, colors: &Colors) -> container::Style {
    container::Style {
        text_color: Some(colors.ink),
        background: Some(background.into()),
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::tests::assert_readable;
    use crate::color::{DARK, LIGHT};
    use crate::theme::{dark, light};

    #[test]
    fn a_selected_tab_uses_the_accent_on_the_soft_accent() {
        let style = tab(true)(&light(), button::Status::Active);

        assert_eq!(style.text_color, LIGHT.accent);
        assert_eq!(style.background, Some(Background::Color(LIGHT.accent_soft)));
    }

    #[test]
    fn an_unselected_tab_stays_quiet_until_hovered() {
        let quiet = tab(false)(&light(), button::Status::Active);
        let hovered = tab(false)(&light(), button::Status::Hovered);

        assert_eq!(quiet.background, None);
        assert_eq!(quiet.text_color, LIGHT.ink_muted);
        assert_eq!(hovered.background, Some(Background::Color(LIGHT.hover)));
        assert_eq!(hovered.text_color, LIGHT.ink);
    }

    #[test]
    fn styles_follow_the_theme() {
        let style = tab(true)(&dark(), button::Status::Active);

        assert_eq!(style.text_color, DARK.accent);
    }

    #[test]
    fn a_focused_field_has_an_accent_border() {
        let style = field(&light(), text_input::Status::Focused { is_hovered: false });

        assert_eq!(style.border.color, LIGHT.accent);
    }

    #[test]
    fn a_badge_tints_the_chrome_with_its_tone() {
        let style = badge(Tone::Danger)(&dark());

        assert_eq!(style.text_color, Some(DARK.danger));
        assert_ne!(style.background, Some(Background::Color(DARK.chrome)));
    }

    #[test]
    fn badge_text_is_readable_in_each_tone() {
        let tones = [
            Tone::Neutral,
            Tone::Accent,
            Tone::Danger,
            Tone::Warning,
            Tone::Success,
            Tone::Info,
        ];
        for (name, theme) in [("light", light()), ("dark", dark())] {
            for tone in tones {
                let style = badge(tone)(&theme);
                let (Some(text), Some(Background::Color(background))) =
                    (style.text_color, style.background)
                else {
                    panic!("the {name} {tone:?} badge has no text color or background color");
                };
                assert_readable(&format!("{name} {tone:?} badge"), text, background);
            }
        }
    }
}
