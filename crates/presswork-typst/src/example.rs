use std::sync::{Arc, LazyLock};

use typst::foundations::Smart;
use typst::layout::{Abs, Margin, PageElem};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt as _};
use typst_layout::PagedDocument;

use crate::clock::today;
use crate::fonts::Fonts;
use crate::render::{PageImage, render_page};
use crate::world::World;

const PAGE_WIDTH: f64 = 240.0;
const PAGE_MARGIN: f64 = 15.0;
const MOST_PAGES: usize = 4;

static FONTS: LazyLock<Arc<Fonts>> = LazyLock::new(|| Arc::new(Fonts::embedded()));

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| {
    let mut library = Library::default();
    library
        .styles
        .set(PageElem::width, Smart::Custom(Abs::pt(PAGE_WIDTH).into()));
    library.styles.set(PageElem::height, Smart::Auto);
    library.styles.set(
        PageElem::margin,
        Smart::Custom(Margin::splat(Some(Smart::Custom(
            Abs::pt(PAGE_MARGIN).into(),
        )))),
    );
    LazyHash::new(library)
});

#[must_use]
pub fn render_example(source: &str, pixel_per_pt: f32) -> Option<Vec<PageImage>> {
    let mut world = World::with_library(FONTS.clone(), today(), LIBRARY.clone());
    world.set_main_text(source);
    let document = typst::compile::<PagedDocument>(&world).output.ok()?;
    Some(
        (0..document.pages().len().min(MOST_PAGES))
            .filter_map(|index| render_page(&document, index, pixel_per_pt, false))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::reference;

    #[test]
    fn an_example_renders_on_a_narrow_page_with_the_height_of_its_content() {
        let pages = render_example("Hello", 2.0).unwrap();

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].width, 480);
        assert!(pages[0].height < 160, "{}", pages[0].height);
    }

    #[test]
    fn an_example_can_set_its_own_page() {
        let pages = render_example("#set page(width: 100pt, height: 50pt)\nHello", 1.0).unwrap();

        assert_eq!((pages[0].width, pages[0].height), (100, 50));
    }

    #[test]
    fn an_example_that_needs_a_file_gives_no_image() {
        assert_eq!(render_example("#image(\"tiger.jpg\")", 1.0), None);
    }

    #[test]
    fn an_example_shows_at_most_four_pages() {
        let pages = render_example(&"Page.\n#pagebreak()\n".repeat(6), 0.5).unwrap();

        assert_eq!(pages.len(), 4);
    }

    #[test]
    fn the_examples_of_the_reference_render() {
        let example = reference()
            .page("calc.pow")
            .unwrap()
            .examples
            .first()
            .unwrap();

        assert!(render_example(&example.source, 1.0).is_some());
    }
}
