use std::borrow::Cow;

pub use iced::Font;
use iced::font::{Family, Weight};

pub const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.ttf");
pub const JETBRAINS_MONO: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
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
        LUCIDE,
    ]
    .into_iter()
    .map(Cow::Borrowed)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttf_parser::{Face, name_id};

    fn face(bytes: &[u8]) -> Face<'_> {
        Face::parse(bytes, 0).unwrap()
    }

    fn family(bytes: &[u8]) -> String {
        let face = face(bytes);
        let names: Vec<_> = face.names().into_iter().collect();
        [name_id::TYPOGRAPHIC_FAMILY, name_id::FAMILY]
            .into_iter()
            .find_map(|id| {
                names
                    .iter()
                    .filter(|name| name.name_id == id)
                    .find_map(ttf_parser::name::Name::to_string)
            })
            .unwrap()
    }

    #[test]
    fn the_interface_fonts_are_inter_in_three_weights() {
        for (bytes, weight) in [
            (INTER_REGULAR, 400),
            (INTER_MEDIUM, 500),
            (INTER_SEMIBOLD, 600),
        ] {
            assert_eq!(family(bytes), UI_FAMILY);
            assert_eq!(face(bytes).weight().to_number(), weight);
        }
    }

    #[test]
    fn the_monospace_font_is_jetbrains_mono() {
        assert_eq!(family(JETBRAINS_MONO), MONO_FAMILY);
    }

    #[test]
    fn the_icon_font_is_lucide() {
        assert_eq!(family(LUCIDE), ICON_FAMILY);
    }

    #[test]
    fn all_gives_each_font_one_time() {
        assert_eq!(all().len(), 5);
    }
}
