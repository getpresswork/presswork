use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

#[derive(Debug)]
pub struct Fonts {
    pub(crate) book: LazyHash<FontBook>,
    pub(crate) fonts: Vec<Font>,
}

impl Fonts {
    #[must_use]
    pub fn embedded() -> Self {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        Self {
            book: LazyHash::new(FontBook::from_fonts(&fonts)),
            fonts,
        }
    }

    #[must_use]
    pub fn families(&self) -> Vec<String> {
        self.book
            .families()
            .map(|(family, _)| family.to_owned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_fonts_include_the_typst_defaults() {
        let families = Fonts::embedded().families();

        for family in [
            "Libertinus Serif",
            "New Computer Modern",
            "DejaVu Sans Mono",
        ] {
            assert!(
                families.iter().any(|name| name == family),
                "{family} is missing from {families:?}"
            );
        }
    }
}
