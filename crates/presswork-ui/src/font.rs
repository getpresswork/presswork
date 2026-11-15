use std::borrow::Cow;

pub use iced::Font;
use iced::font::{Family, Style, Weight};

pub const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.ttf");
pub const JETBRAINS_MONO: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
pub const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");
pub const JETBRAINS_MONO_ITALIC: &[u8] = include_bytes!("../fonts/JetBrainsMono-Italic.ttf");
pub const LUCIDE: &[u8] = lucide_icons::LUCIDE_FONT_BYTES;

pub const UI_FAMILY: &str = "Inter";
pub const MONO_FAMILY: &str = "JetBrains Mono";
pub const ICON_FAMILY: &str = "lucide";

pub const UI: Font = Font {
    family: Family::Name(UI_FAMILY),
    ..Font::DEFAULT
};
pub const UI_MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..UI
};
pub const UI_SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..UI
};
pub const MONO: Font = Font {
    family: Family::Name(MONO_FAMILY),
    ..Font::DEFAULT
};
pub const MONO_BOLD: Font = Font {
    weight: Weight::Bold,
    ..MONO
};
pub const MONO_ITALIC: Font = Font {
    style: Style::Italic,
    ..MONO
};
pub const ICONS: Font = Font {
    family: Family::Name(ICON_FAMILY),
    ..Font::DEFAULT
};

#[must_use]
pub fn all() -> Vec<Cow<'static, [u8]>> {
    [
        INTER_REGULAR,
        INTER_MEDIUM,
        INTER_SEMIBOLD,
        JETBRAINS_MONO,
        JETBRAINS_MONO_BOLD,
        JETBRAINS_MONO_ITALIC,
        LUCIDE,
    ]
    .into_iter()
    .map(Cow::Borrowed)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use skrifa::attribute::Weight as FontWeight;
    use skrifa::string::StringId;
    use skrifa::{FontRef, MetadataProvider};

    fn font(bytes: &[u8]) -> FontRef<'_> {
        FontRef::new(bytes).unwrap()
    }

    fn family(bytes: &[u8]) -> String {
        let font = font(bytes);
        [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
            .into_iter()
            .find_map(|id| font.localized_strings(id).english_or_first())
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_interface_fonts_are_inter_in_three_weights() {
        for (bytes, weight) in [
            (INTER_REGULAR, FontWeight::NORMAL),
            (INTER_MEDIUM, FontWeight::MEDIUM),
            (INTER_SEMIBOLD, FontWeight::SEMI_BOLD),
        ] {
            assert_eq!(family(bytes), UI_FAMILY);
            assert_eq!(font(bytes).attributes().weight, weight);
        }
    }

    #[test]
    fn the_monospace_font_is_jetbrains_mono() {
        assert_eq!(family(JETBRAINS_MONO), MONO_FAMILY);
    }

    #[test]
    fn the_monospace_font_has_bold_and_italic_faces() {
        assert_eq!(family(JETBRAINS_MONO_BOLD), MONO_FAMILY);
        assert_eq!(
            font(JETBRAINS_MONO_BOLD).attributes().weight,
            FontWeight::BOLD
        );
        assert_eq!(family(JETBRAINS_MONO_ITALIC), MONO_FAMILY);
        assert_eq!(
            font(JETBRAINS_MONO_ITALIC).attributes().style,
            skrifa::attribute::Style::Italic
        );
    }

    #[test]
    fn the_icon_font_is_lucide() {
        assert_eq!(family(LUCIDE), ICON_FAMILY);
    }

    #[test]
    fn all_gives_each_font_one_time() {
        assert_eq!(all().len(), 7);
    }
}
