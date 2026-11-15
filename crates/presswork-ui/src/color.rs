pub use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub page: Color,
    pub chrome: Color,
    pub raised: Color,
    pub hover: Color,
    pub rule: Color,
    pub rule_strong: Color,
    pub ink: Color,
    pub ink_muted: Color,
    pub ink_faint: Color,
    pub accent: Color,
    pub accent_strong: Color,
    pub accent_soft: Color,
    pub on_accent: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
    pub editor: EditorColors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorColors {
    pub current_line: Color,
    pub selection: Color,
    pub line_number: Color,
    pub comment: Color,
    pub keyword: Color,
    pub function: Color,
    pub string: Color,
    pub number: Color,
    pub operator: Color,
    pub reference: Color,
    pub error: Color,
}

pub const LIGHT: Colors = Colors {
    page: rgb(0xFB, 0xFA, 0xF7),
    chrome: rgb(0xF3, 0xF0, 0xE9),
    raised: rgb(0xFF, 0xFF, 0xFF),
    hover: rgb(0xE9, 0xE5, 0xDC),
    rule: rgb(0xE2, 0xDD, 0xD3),
    rule_strong: rgb(0xCE, 0xC7, 0xB9),
    ink: rgb(0x1F, 0x1D, 0x1A),
    ink_muted: rgb(0x5F, 0x59, 0x50),
    ink_faint: rgb(0x8E, 0x87, 0x7D),
    accent: rgb(0x8C, 0x2F, 0x39),
    accent_strong: rgb(0x74, 0x25, 0x2E),
    accent_soft: rgb(0xF2, 0xE3, 0xE4),
    on_accent: rgb(0xFF, 0xFF, 0xFF),
    danger: rgb(0xA8, 0x23, 0x1B),
    warning: rgb(0x86, 0x50, 0x00),
    success: rgb(0x2C, 0x66, 0x4A),
    info: rgb(0x1D, 0x4E, 0x89),
    editor: EditorColors {
        current_line: rgb(0xF4, 0xF1, 0xEB),
        selection: rgb(0xF2, 0xE3, 0xE4),
        line_number: rgb(0x77, 0x6F, 0x64),
        comment: rgb(0x6B, 0x63, 0x58),
        keyword: rgb(0x8C, 0x2F, 0x39),
        function: rgb(0x1D, 0x4E, 0x89),
        string: rgb(0x2C, 0x66, 0x4A),
        number: rgb(0x86, 0x50, 0x00),
        operator: rgb(0x5F, 0x59, 0x50),
        reference: rgb(0x7A, 0x3B, 0x69),
        error: rgb(0xA8, 0x23, 0x1B),
    },
};

pub const DARK: Colors = Colors {
    page: rgb(0x1A, 0x19, 0x17),
    chrome: rgb(0x21, 0x1F, 0x1D),
    raised: rgb(0x2A, 0x28, 0x25),
    hover: rgb(0x31, 0x2E, 0x2B),
    rule: rgb(0x3A, 0x36, 0x32),
    rule_strong: rgb(0x4A, 0x45, 0x40),
    ink: rgb(0xEC, 0xE7, 0xDF),
    ink_muted: rgb(0xA9, 0xA1, 0x96),
    ink_faint: rgb(0x7D, 0x76, 0x6C),
    accent: rgb(0xE0, 0x7A, 0x84),
    accent_strong: rgb(0xEA, 0x97, 0xA0),
    accent_soft: rgb(0x3B, 0x25, 0x28),
    on_accent: rgb(0x1A, 0x19, 0x17),
    danger: rgb(0xF2, 0x86, 0x7C),
    warning: rgb(0xE3, 0xAE, 0x5E),
    success: rgb(0x7F, 0xC2, 0xA0),
    info: rgb(0x7F, 0xB0, 0xE8),
    editor: EditorColors {
        current_line: rgb(0x21, 0x1F, 0x1D),
        selection: rgb(0x3B, 0x25, 0x28),
        line_number: rgb(0x91, 0x8A, 0x7E),
        comment: rgb(0xA3, 0x9B, 0x8E),
        keyword: rgb(0xE0, 0x7A, 0x84),
        function: rgb(0x7F, 0xB0, 0xE8),
        string: rgb(0x7F, 0xC2, 0xA0),
        number: rgb(0xE3, 0xAE, 0x5E),
        operator: rgb(0xA9, 0xA1, 0x96),
        reference: rgb(0xCB, 0x9B, 0xC2),
        error: rgb(0xF2, 0x86, 0x7C),
    },
};

#[must_use]
pub const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color {
        r: channel(red),
        g: channel(green),
        b: channel(blue),
        a: 1.0,
    }
}

const fn channel(value: u8) -> f32 {
    value as f32 / 255.0
}

#[must_use]
pub fn mix(from: Color, to: Color, amount: f32) -> Color {
    let blend = |a: f32, b: f32| a + (b - a) * amount;
    Color {
        r: blend(from.r, to.r),
        g: blend(from.g, to.g),
        b: blend(from.b, to.b),
        a: blend(from.a, to.a),
    }
}

#[must_use]
pub fn contrast(foreground: Color, background: Color) -> f32 {
    let (lighter, darker) = {
        let a = luminance(foreground);
        let b = luminance(background);
        if a > b { (a, b) } else { (b, a) }
    };
    (lighter + 0.05) / (darker + 0.05)
}

fn luminance(color: Color) -> f32 {
    let linear = |value: f32| {
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const TEXT_MINIMUM: f32 = 4.5;

    pub(crate) fn assert_readable(name: &str, foreground: Color, background: Color) {
        let ratio = contrast(foreground, background);
        assert!(
            ratio >= TEXT_MINIMUM,
            "{name}: contrast {ratio:.2} is below {TEXT_MINIMUM}"
        );
    }

    #[test]
    fn rgb_makes_an_opaque_color() {
        let color = rgb(0xFF, 0x80, 0x00);

        assert!((color.r - 1.0).abs() < f32::EPSILON);
        assert!((color.g - 128.0 / 255.0).abs() < f32::EPSILON);
        assert!(color.b.abs() < f32::EPSILON);
        assert!((color.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn contrast_of_black_on_white_is_twenty_one() {
        let ratio = contrast(rgb(0x00, 0x00, 0x00), rgb(0xFF, 0xFF, 0xFF));

        assert!((ratio - 21.0).abs() < 0.01);
    }

    #[test]
    fn mix_moves_between_two_colors() {
        let middle = mix(rgb(0x00, 0x00, 0x00), rgb(0xFF, 0xFF, 0xFF), 0.5);

        assert!((middle.r - 0.5).abs() < 0.01);
    }

    #[test]
    fn editor_text_colors_are_readable_on_all_editor_backgrounds() {
        for (theme, colors) in [("light", &LIGHT), ("dark", &DARK)] {
            let editor = colors.editor;
            for (surface, background) in [
                ("page", colors.page),
                ("current line", editor.current_line),
                ("selection", editor.selection),
            ] {
                for (role, foreground) in [
                    ("ink", colors.ink),
                    ("comment", editor.comment),
                    ("keyword", editor.keyword),
                    ("function", editor.function),
                    ("string", editor.string),
                    ("number", editor.number),
                    ("operator", editor.operator),
                    ("reference", editor.reference),
                    ("error", editor.error),
                ] {
                    assert_readable(
                        &format!("{theme} {role} on {surface}"),
                        foreground,
                        background,
                    );
                }
            }
            assert_readable(
                &format!("{theme} line number on page"),
                editor.line_number,
                colors.page,
            );
        }
    }

    #[test]
    fn all_text_colors_are_readable_on_all_surfaces() {
        for (theme, colors) in [("light", &LIGHT), ("dark", &DARK)] {
            for (surface, background) in [
                ("page", colors.page),
                ("chrome", colors.chrome),
                ("raised", colors.raised),
            ] {
                for (role, foreground) in [
                    ("ink", colors.ink),
                    ("ink_muted", colors.ink_muted),
                    ("accent", colors.accent),
                    ("danger", colors.danger),
                    ("warning", colors.warning),
                    ("success", colors.success),
                    ("info", colors.info),
                ] {
                    assert_readable(
                        &format!("{theme} {role} on {surface}"),
                        foreground,
                        background,
                    );
                }
            }
            assert_readable(
                &format!("{theme} on_accent on accent"),
                colors.on_accent,
                colors.accent,
            );
            assert_readable(
                &format!("{theme} accent on accent_soft"),
                colors.accent,
                colors.accent_soft,
            );
        }
    }
}
