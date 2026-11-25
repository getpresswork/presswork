use std::path::{Path, PathBuf};
use std::sync::Arc;

use typst::World as _;
use typst::introspection::PagedPosition;
use typst::layout::{Abs, Frame, FrameItem, Point};
use typst::syntax::{FileId, LinkedNode, Side, Source, Span, SyntaxKind, VirtualRoot};
use typst_ide::IdeWorld;
use typst_layout::PagedDocument;

use crate::world::{World, file_id};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PagePoint {
    pub page: usize,
    pub x: f64,
    pub y: f64,
}

impl From<PagedPosition> for PagePoint {
    fn from(position: PagedPosition) -> Self {
        Self {
            page: position.page.get() - 1,
            x: position.point.x.to_pt(),
            y: position.point.y.to_pt(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Jump {
    Main(usize),
    File(PathBuf, usize),
    Page(PagePoint),
}

#[derive(Debug, Clone)]
pub struct Analysis {
    world: Arc<World>,
    document: Option<Arc<PagedDocument>>,
}

impl Analysis {
    pub(crate) fn new(world: World, document: Option<PagedDocument>) -> Self {
        Self {
            world: Arc::new(world),
            document: document.map(Arc::new),
        }
    }

    #[must_use]
    pub fn shared_document(&self) -> Option<&Arc<PagedDocument>> {
        self.document.as_ref()
    }

    pub(crate) fn source_with(&self, file: Option<&Path>, text: &str) -> Option<(World, Source)> {
        let mut world = (*self.world).clone();
        let id = match file {
            None => {
                world.set_main_text(text);
                world.main()
            }
            Some(path) => {
                world.set_text(path, text);
                file_id(path)?
            }
        };
        let source = world.source(id).ok()?;
        Some((world, source))
    }

    pub(crate) fn world(&self) -> &World {
        &self.world
    }

    pub(crate) fn document(&self) -> Option<&PagedDocument> {
        self.document.as_deref()
    }

    #[must_use]
    pub fn jump_from_click(&self, point: PagePoint) -> Option<Jump> {
        let document = self.document.as_deref()?;
        let page = document.pages().get(point.page)?;
        let click = Point::new(Abs::pt(point.x), Abs::pt(point.y));
        let world: &dyn IdeWorld = &*self.world;
        let jump = match typst_ide::jump_from_click_in_frame(world, document, &page.frame, click)? {
            typst_ide::Jump::Url(_) => {
                let mut frame = page.frame.clone();
                remove_links(&mut frame);
                typst_ide::jump_from_click_in_frame(world, document, &frame, click)?
            }
            jump => jump,
        };
        match jump {
            typst_ide::Jump::File(id, offset) if id == self.world.main() => {
                Some(Jump::Main(offset))
            }
            typst_ide::Jump::File(id, offset) => Some(Jump::File(project_path(id)?, offset)),
            typst_ide::Jump::Position(position) => Some(Jump::Page(position.into())),
            typst_ide::Jump::Url(_) => None,
        }
    }

    #[must_use]
    pub fn jump_from_cursor(&self, file: Option<&Path>, offset: usize) -> Vec<PagePoint> {
        let id = match file {
            None => Some(self.world.main()),
            Some(path) => file_id(path),
        };
        match id.map(|id| self.world.source(id)) {
            Some(Ok(source)) => self.points(&source, offset),
            _ => Vec::new(),
        }
    }

    fn points(&self, source: &Source, offset: usize) -> Vec<PagePoint> {
        let Some(document) = self.document.as_deref() else {
            return Vec::new();
        };
        let root = LinkedNode::new(source.root());
        let Some(node) = root
            .leaf_at(offset, Side::Before)
            .filter(is_text)
            .or_else(|| root.leaf_at(offset, Side::After).filter(is_text))
        else {
            return Vec::new();
        };
        let within = u16::try_from(offset.saturating_sub(node.offset())).unwrap_or(u16::MAX);
        let found: Vec<(usize, Glyph)> = document
            .pages()
            .iter()
            .enumerate()
            .filter_map(|(page, content)| {
                glyph_at(&content.frame, node.span(), within).map(|glyph| (page, glyph))
            })
            .collect();
        let Some(nearest) = found.iter().map(|(_, glyph)| glyph.at).max() else {
            return Vec::new();
        };
        found
            .into_iter()
            .filter(|(_, glyph)| glyph.at == nearest)
            .map(|(page, glyph)| PagePoint {
                page,
                x: glyph.point.x.to_pt(),
                y: glyph.point.y.to_pt(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
struct Glyph {
    at: u16,
    point: Point,
}

fn is_text(node: &LinkedNode<'_>) -> bool {
    matches!(node.kind(), SyntaxKind::Text | SyntaxKind::MathText)
}

fn glyph_at(frame: &Frame, span: Span, within: u16) -> Option<Glyph> {
    let mut nearest: Option<Glyph> = None;
    for (position, item) in frame.items() {
        let found = match item {
            FrameItem::Group(group) => glyph_at(&group.frame, span, within).map(|glyph| Glyph {
                point: *position + glyph.point.transform(group.transform),
                ..glyph
            }),
            FrameItem::Text(text) => {
                let mut x = position.x;
                let mut found: Option<Glyph> = None;
                for glyph in &text.glyphs {
                    let (glyph_span, at) = glyph.span;
                    if glyph_span == span && at <= within && found.is_none_or(|best| at >= best.at)
                    {
                        found = Some(Glyph {
                            at,
                            point: Point::new(x, position.y - text.size / 2.0),
                        });
                    }
                    x += glyph.x_advance.at(text.size);
                }
                found
            }
            _ => None,
        };
        if let Some(glyph) = found
            && nearest.is_none_or(|best| glyph.at >= best.at)
        {
            nearest = Some(glyph);
        }
    }
    nearest
}

fn remove_links(frame: &mut Frame) {
    frame.retain(|item| match item {
        FrameItem::Link(..) => false,
        FrameItem::Group(group) => {
            remove_links(&mut group.frame);
            true
        }
        _ => true,
    });
}

pub(crate) fn project_path(id: FileId) -> Option<PathBuf> {
    let rooted = id.get();
    match rooted.root() {
        VirtualRoot::Project => Some(PathBuf::from(rooted.vpath().get_without_slash())),
        VirtualRoot::Package(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;

    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;

    const TWO_PAGES: &str = "First page.\n#pagebreak()\nSecond page.";

    fn today() -> Option<Datetime> {
        Datetime::from_ymd(2026, 11, 18)
    }

    fn analysis(text: &str) -> Analysis {
        let world = Mutex::new(World::new(Arc::new(Fonts::embedded()), today()));
        analyze(&world, text).analysis.unwrap()
    }

    fn start_of(text: &str, word: &str) -> usize {
        text.find(word).unwrap()
    }

    fn inside(point: PagePoint) -> PagePoint {
        PagePoint {
            x: point.x + 0.5,
            ..point
        }
    }

    #[test]
    fn the_cursor_in_the_text_of_the_second_page_is_on_the_second_page() {
        let points = analysis(TWO_PAGES).jump_from_cursor(None, start_of(TWO_PAGES, "Second") + 2);

        assert_eq!(
            points.iter().map(|point| point.page).collect::<Vec<_>>(),
            [1]
        );
    }

    #[test]
    fn a_cursor_outside_of_text_has_no_page_position() {
        let text = "#let name = 1\nText.";

        assert_eq!(analysis(text).jump_from_cursor(None, 2), []);
    }

    #[test]
    fn the_page_point_of_a_cursor_is_at_its_character() {
        let text = "Some words in one line.";
        let analysis = analysis(text);

        let first = analysis.jump_from_cursor(None, start_of(text, "Some"))[0];
        let later = analysis.jump_from_cursor(None, start_of(text, "one"))[0];

        assert!(later.x > first.x + 20.0, "{first:?} {later:?}");
        assert!((later.y - first.y).abs() < 0.01, "{first:?} {later:?}");
    }

    #[test]
    fn a_paragraph_on_two_pages_gives_only_the_page_of_the_cursor() {
        let text = format!(
            "#set page(height: 80pt, margin: 10pt)\n{}",
            "word ".repeat(200)
        );

        let points = analysis(&text).jump_from_cursor(None, text.len() - 3);

        assert_eq!(points.len(), 1, "{points:?}");
        assert!(points[0].page > 0, "{points:?}");
    }

    #[test]
    fn a_click_on_a_word_gives_its_position_in_the_main_file() {
        let analysis = analysis(TWO_PAGES);
        let offset = start_of(TWO_PAGES, "Second") + 2;
        let point = analysis.jump_from_cursor(None, offset)[0];

        assert_eq!(
            analysis.jump_from_click(inside(point)),
            Some(Jump::Main(offset))
        );
    }

    #[test]
    fn a_click_on_the_empty_margin_gives_no_jump() {
        let point = PagePoint {
            page: 0,
            x: 1.0,
            y: 1.0,
        };

        assert_eq!(analysis(TWO_PAGES).jump_from_click(point), None);
    }

    #[test]
    fn a_click_on_a_page_that_does_not_exist_gives_no_jump() {
        let point = PagePoint {
            page: 5,
            x: 100.0,
            y: 100.0,
        };

        assert_eq!(analysis(TWO_PAGES).jump_from_click(point), None);
    }

    #[test]
    fn a_click_on_a_web_link_gives_the_position_of_its_text() {
        let text = "#link(\"https://typst.app\")[Typst]";
        let analysis = analysis(text);
        let offset = start_of(text, "Typst]") + 1;
        let point = analysis.jump_from_cursor(None, offset)[0];

        assert_eq!(
            analysis.jump_from_click(inside(point)),
            Some(Jump::Main(offset))
        );
    }

    #[test]
    fn a_click_on_a_link_to_a_label_gives_the_page_of_the_label() {
        let text = "#link(<end>)[Go]\n#pagebreak()\n= The end <end>";
        let analysis = analysis(text);
        let point = analysis.jump_from_cursor(None, start_of(text, "Go") + 1)[0];
        let jump = analysis.jump_from_click(inside(point));

        assert!(
            matches!(jump, Some(Jump::Page(PagePoint { page: 1, .. }))),
            "{jump:?}"
        );
    }

    #[test]
    fn a_click_on_included_text_gives_the_file_and_the_position() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("main.typ"), "#include \"part.typ\"").unwrap();
        let part = "#let x = 1\nIncluded words.";
        fs::write(folder.path().join("part.typ"), part).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            today(),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        let text = fs::read_to_string(folder.path().join("main.typ")).unwrap();
        let analysis = analyze(&world, &text).analysis.unwrap();
        let source = analysis
            .world
            .source(crate::world::file_id(Path::new("part.typ")).unwrap())
            .unwrap();
        let offset = start_of(part, "words");
        let point = analysis.points(&source, offset)[0];

        assert_eq!(
            analysis.jump_from_click(inside(point)),
            Some(Jump::File(PathBuf::from("part.typ"), offset))
        );
    }

    #[test]
    fn the_cursor_in_an_included_file_is_on_the_page_of_its_text() {
        let folder = tempfile::tempdir().unwrap();
        let main = "First page.\n#pagebreak()\n#include \"part.typ\"";
        let part = "Included words.";
        fs::write(folder.path().join("main.typ"), main).unwrap();
        fs::write(folder.path().join("part.typ"), part).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            today(),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        let analysis = analyze(&world, main).analysis.unwrap();

        let points =
            analysis.jump_from_cursor(Some(Path::new("part.typ")), start_of(part, "words"));

        assert_eq!(
            points.iter().map(|point| point.page).collect::<Vec<_>>(),
            [1]
        );
    }
}
