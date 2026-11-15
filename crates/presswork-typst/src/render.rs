use typst::utils::Scalar;
use typst_layout::PagedDocument;
use typst_render::RenderOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[must_use]
pub fn render_page(document: &PagedDocument, index: usize, pixel_per_pt: f32) -> Option<PageImage> {
    let page = document.pages().get(index)?;
    let options = RenderOptions {
        pixel_per_pt: Scalar::new(f64::from(pixel_per_pt)),
        ..RenderOptions::default()
    };
    let pixmap = typst_render::render(page, &options);
    Some(PageImage {
        width: pixmap.width(),
        height: pixmap.height(),
        rgba: pixmap.take(),
    })
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
        let mut world = World::new(Arc::new(Fonts::embedded()), today);
        world.set_main_text(text);
        compile(&world).document.unwrap()
    }

    #[test]
    fn a_page_renders_at_the_requested_scale() {
        let document = document("#set page(width: 100pt, height: 50pt)\nText.");

        let image = render_page(&document, 0, 2.0).unwrap();

        assert_eq!((image.width, image.height), (200, 100));
        assert_eq!(image.rgba.len(), 200 * 100 * 4);
        assert_eq!(&image.rgba[..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn a_page_that_does_not_exist_gives_no_image() {
        let document = document("One page.");

        assert_eq!(render_page(&document, 1, 1.0), None);
    }
}
