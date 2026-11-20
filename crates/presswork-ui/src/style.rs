use iced::widget::{button, container, rule, text, text_input};
use iced::{Background, Border, Color, Shadow, Theme};

use crate::color::{Colors, mix};
use crate::radius;
use crate::theme::colors;

const BADGE_TINT: f32 = 0.14;
const NOTICE_HOVER_TINT: f32 = 0.22;
const PRESSED_CLOSE: f32 = 0.9;
const POPUP_SHADOW: f32 = 0.16;
const SCRIM: f32 = 0.32;

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
pub fn paper(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: colors(theme).rule,
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.14,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
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

pub fn notice(tone: Tone) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let colors = colors(theme);
        let color = tone.color(colors);
        let tint = match status {
            button::Status::Hovered | button::Status::Pressed => NOTICE_HOVER_TINT,
            button::Status::Active | button::Status::Disabled => BADGE_TINT,
        };
        button::Style {
            background: Some(mix(colors.chrome, color, tint).into()),
            text_color: color,
            border: Border {
                radius: radius::SMALL.into(),
                ..Border::default()
            },
            ..button::Style::default()
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

pub fn list_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let colors = colors(theme);
        let (background, text_color) = match (selected, status) {
            (true, _) => (Some(colors.accent_soft), colors.accent),
            (false, button::Status::Hovered | button::Status::Pressed) => {
                (Some(colors.hover), colors.ink)
            }
            (false, button::Status::Disabled) => (None, colors.ink_faint),
            (false, button::Status::Active) => (None, colors.ink),
        };
        button::Style {
            border: Border {
                radius: radius::SMALL.into(),
                ..Border::default()
            },
            ..flat(background, text_color)
        }
    }
}

#[must_use]
pub fn window_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = colors(theme);
    let background = match status {
        button::Status::Hovered => Some(colors.hover),
        button::Status::Pressed => Some(colors.rule_strong),
        button::Status::Active | button::Status::Disabled => None,
    };
    flat(background, colors.ink)
}

#[must_use]
pub fn close_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = colors(theme);
    match status {
        button::Status::Hovered => flat(Some(colors.close), colors.on_close),
        button::Status::Pressed => flat(
            Some(Color {
                a: PRESSED_CLOSE,
                ..colors.close
            }),
            colors.on_close,
        ),
        button::Status::Active | button::Status::Disabled => window_button(theme, status),
    }
}

#[must_use]
pub fn menu_item(theme: &Theme, status: button::Status) -> button::Style {
    let colors = colors(theme);
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (Some(colors.hover), colors.ink),
        button::Status::Disabled => (None, colors.ink_faint),
        button::Status::Active => (None, colors.ink),
    };
    button::Style {
        border: Border {
            radius: radius::SMALL.into(),
            ..Border::default()
        },
        ..flat(background, text_color)
    }
}

#[must_use]
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let colors = colors(theme);
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (colors.accent_strong, colors.on_accent)
        }
        button::Status::Active => (colors.accent, colors.on_accent),
        button::Status::Disabled => (colors.hover, colors.ink_faint),
    };
    button::Style {
        border: Border {
            radius: radius::MEDIUM.into(),
            ..Border::default()
        },
        ..flat(Some(background), text_color)
    }
}

#[must_use]
pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let colors = colors(theme);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => colors.hover,
        button::Status::Active | button::Status::Disabled => colors.raised,
    };
    button::Style {
        border: Border {
            color: colors.rule_strong,
            width: 1.0,
            radius: radius::MEDIUM.into(),
        },
        ..flat(Some(background), colors.ink)
    }
}

fn flat(background: Option<Color>, text_color: Color) -> button::Style {
    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

#[must_use]
pub fn popup(theme: &Theme) -> container::Style {
    let colors = colors(theme);
    container::Style {
        text_color: Some(colors.ink),
        background: Some(colors.raised.into()),
        border: Border {
            color: colors.rule,
            width: 1.0,
            radius: radius::MEDIUM.into(),
        },
        shadow: Shadow {
            color: Color {
                a: POPUP_SHADOW,
                ..Color::BLACK
            },
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

#[must_use]
pub fn scrim(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(
            Color {
                a: SCRIM,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    }
}

#[must_use]
pub fn outline(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: colors(theme).rule,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container::Style::default()
    }
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
    fn a_list_item_uses_the_ink_and_the_hover_color() {
        let quiet = list_item(false)(&light(), button::Status::Active);
        let hovered = list_item(false)(&light(), button::Status::Hovered);

        assert_eq!(quiet.background, None);
        assert_eq!(quiet.text_color, LIGHT.ink);
        assert_eq!(hovered.background, Some(Background::Color(LIGHT.hover)));
        assert_eq!(quiet.border.radius, radius::SMALL.into());
    }

    #[test]
    fn a_selected_list_item_uses_the_accent_on_the_soft_accent() {
        let style = list_item(true)(&dark(), button::Status::Hovered);

        assert_eq!(style.text_color, DARK.accent);
        assert_eq!(style.background, Some(Background::Color(DARK.accent_soft)));
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
    fn a_notice_shows_more_tint_when_hovered() {
        let quiet = notice(Tone::Danger)(&light(), button::Status::Active);
        let hovered = notice(Tone::Danger)(&light(), button::Status::Hovered);

        assert_eq!(quiet.text_color, LIGHT.danger);
        assert_ne!(quiet.background, hovered.background);
        assert_eq!(quiet.background, badge(Tone::Danger)(&light()).background);
    }

    #[test]
    fn window_buttons_show_the_hover_color_under_the_pointer() {
        let quiet = window_button(&light(), button::Status::Active);
        let hovered = window_button(&light(), button::Status::Hovered);

        assert_eq!(quiet.background, None);
        assert_eq!(quiet.text_color, LIGHT.ink);
        assert_eq!(hovered.background, Some(Background::Color(LIGHT.hover)));
    }

    #[test]
    fn the_close_button_becomes_red_under_the_pointer() {
        for (theme, colors) in [(light(), LIGHT), (dark(), DARK)] {
            let quiet = close_button(&theme, button::Status::Active);
            let hovered = close_button(&theme, button::Status::Hovered);

            assert_eq!(quiet.background, None);
            assert_eq!(hovered.background, Some(Background::Color(colors.close)));
            assert_eq!(hovered.text_color, colors.on_close);
        }
    }

    #[test]
    fn a_popup_is_raised_with_a_rule_and_a_shadow() {
        let style = popup(&light());

        assert_eq!(style.background, Some(Background::Color(LIGHT.raised)));
        assert_eq!(style.border.color, LIGHT.rule);
        assert_eq!(style.border.width, 1.0);
        assert!(style.shadow.blur_radius > 0.0);
    }

    #[test]
    fn a_menu_item_is_faint_when_it_is_not_available() {
        let hovered = menu_item(&light(), button::Status::Hovered);
        let disabled = menu_item(&light(), button::Status::Disabled);

        assert_eq!(hovered.background, Some(Background::Color(LIGHT.hover)));
        assert_eq!(hovered.text_color, LIGHT.ink);
        assert_eq!(disabled.text_color, LIGHT.ink_faint);
    }

    #[test]
    fn the_primary_button_uses_the_accent() {
        let quiet = primary(&light(), button::Status::Active);
        let hovered = primary(&light(), button::Status::Hovered);

        assert_eq!(quiet.background, Some(Background::Color(LIGHT.accent)));
        assert_eq!(quiet.text_color, LIGHT.on_accent);
        assert_eq!(
            hovered.background,
            Some(Background::Color(LIGHT.accent_strong))
        );
    }

    #[test]
    fn a_disabled_primary_button_is_quiet() {
        let style = primary(&light(), button::Status::Disabled);

        assert_eq!(style.background, Some(Background::Color(LIGHT.hover)));
        assert_eq!(style.text_color, LIGHT.ink_faint);
    }

    #[test]
    fn a_secondary_button_is_raised_with_a_rule() {
        let quiet = secondary(&light(), button::Status::Active);
        let hovered = secondary(&light(), button::Status::Hovered);

        assert_eq!(quiet.background, Some(Background::Color(LIGHT.raised)));
        assert_eq!(quiet.border.color, LIGHT.rule_strong);
        assert_eq!(quiet.text_color, LIGHT.ink);
        assert_eq!(hovered.background, Some(Background::Color(LIGHT.hover)));
    }

    #[test]
    fn the_scrim_darkens_the_window_partly() {
        let Some(Background::Color(color)) = scrim(&light()).background else {
            panic!("the scrim has no color");
        };

        assert!(color.a > 0.0 && color.a < 1.0, "{color:?}");
    }

    #[test]
    fn the_window_outline_is_a_rule() {
        let style = outline(&dark());

        assert_eq!(style.border.color, DARK.rule);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.background, None);
    }

    #[test]
    fn paper_has_a_rule_and_a_soft_shadow() {
        let style = paper(&light());

        assert_eq!(style.border.color, LIGHT.rule);
        assert_eq!(style.border.width, 1.0);
        assert!(style.shadow.blur_radius > 0.0);
        assert!(style.shadow.color.a < 0.3);
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
