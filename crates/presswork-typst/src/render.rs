use typst::layout::Abs;
use typst::utils::{Scalar, hash128};
use typst_layout::{Page, PagedDocument};
use typst_render::RenderOptions;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub bleed: Insets,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
    pub bleed: Insets,
}

#[must_use]
pub fn page_sizes(document: &PagedDocument, bleed: bool) -> Vec<PageSize> {
    document
        .pages()
        .iter()
        .map(|page| {
            let shown = shown_bleed(page, bleed);
            PageSize {
                width: points(page.frame.width()) + shown.left + shown.right,
                height: points(page.frame.height()) + shown.top + shown.bottom,
                bleed: shown,
            }
        })
        .collect()
}

#[must_use]
pub fn page_key(document: &PagedDocument, index: usize, bleed: bool) -> Option<u128> {
    let page = document.pages().get(index)?;
    Some(hash128(&(page, bleed)))
}

#[must_use]
pub fn render_page(
    document: &PagedDocument,
    index: usize,
    pixel_per_pt: f32,
    bleed: bool,
) -> Option<PageImage> {
    let page = document.pages().get(index)?;
    let options = RenderOptions {
        pixel_per_pt: Scalar::new(f64::from(pixel_per_pt)),
        render_bleed: bleed,
    };
    let pixmap = typst_render::render(page, &options);
    Some(PageImage {
        width: pixmap.width(),
        height: pixmap.height(),
        rgba: pixmap.take(),
        bleed: shown_bleed(page, bleed),
    })
}

fn shown_bleed(page: &Page, bleed: bool) -> Insets {
    if bleed {
        Insets {
            left: points(page.bleed.left),
            top: points(page.bleed.top),
            right: points(page.bleed.right),
            bottom: points(page.bleed.bottom),
        }
    } else {
        Insets::default()
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "page lengths are far below the limits of f32"
)]
fn points(length: Abs) -> f32 {
    length.to_pt() as f32
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use typst::foundations::Datetime;

    use super::*;
    use crate::compile::compile;
    use crate::fonts::Fonts;
    use crate::world::World;

    fn document(text: &str) -> PagedDocument {
        let today = Datetime::from_ymd(2026, 11, 15).unwrap();
        let mut world = World::new(Arc::new(Fonts::embedded()), Some(today));
        world.set_main_text(text);
        compile(&world).document.unwrap()
    }

    #[test]
    fn a_page_renders_at_the_requested_scale() {
        let document = document("#set page(width: 100pt, height: 50pt)\nText.");

        let image = render_page(&document, 0, 2.0, false).unwrap();

        assert_eq!((image.width, image.height), (200, 100));
        assert_eq!(image.rgba.len(), 200 * 100 * 4);
        assert_eq!(&image.rgba[..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn page_sizes_include_the_bleed_only_on_request() {
        let document = document("#set page(width: 100pt, height: 50pt, bleed: 5pt)\nText.");

        let plain = page_sizes(&document, false);
        let bled = page_sizes(&document, true);

        assert_eq!(
            plain,
            [PageSize {
                width: 100.0,
                height: 50.0,
                bleed: Insets::default()
            }]
        );
        assert_eq!(
            bled,
            [PageSize {
                width: 110.0,
                height: 60.0,
                bleed: Insets {
                    left: 5.0,
                    top: 5.0,
                    right: 5.0,
                    bottom: 5.0
                }
            }]
        );
    }

    #[test]
    fn a_page_key_changes_only_with_the_page() {
        let before = document("One.\n#pagebreak()\nTwo.");
        let after = document("One.\n#pagebreak()\nThree.");

        let key =
            |document: &PagedDocument, index, bleed| page_key(document, index, bleed).unwrap();

        assert_eq!(key(&before, 0, false), key(&after, 0, false));
        assert_ne!(key(&before, 1, false), key(&after, 1, false));
        assert_ne!(key(&before, 0, false), key(&before, 0, true));
        assert_eq!(page_key(&before, 2, false), None);
    }

    #[test]
    fn a_page_that_does_not_exist_gives_no_image() {
        let document = document("One page.");

        assert_eq!(render_page(&document, 1, 1.0, false), None);
    }

    #[test]
    fn the_bleed_renders_around_the_page_on_request() {
        let document = document("#set page(width: 100pt, height: 50pt, bleed: 5pt)\nText.");

        let plain = render_page(&document, 0, 1.0, false).unwrap();
        let bled = render_page(&document, 0, 1.0, true).unwrap();

        assert_eq!((plain.width, plain.height), (100, 50));
        assert_eq!(plain.bleed, Insets::default());
        assert_eq!((bled.width, bled.height), (110, 60));
        assert_eq!(
            bled.bleed,
            Insets {
                left: 5.0,
                top: 5.0,
                right: 5.0,
                bottom: 5.0
            }
        );
    }
}
