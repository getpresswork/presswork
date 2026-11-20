use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use typst::foundations::{Content, NativeElement as _, Smart, StyleChain};
use typst::introspection::Introspector as _;
use typst::math::EquationElem;
use typst::model::{FigureElem, FigureKind, HeadingElem, Numbering, TableElem};
use typst::syntax::Span;
use typst::text::RawElem;
use typst::{World as _, WorldExt as _};
use typst_layout::PagedDocument;

use crate::ide::{Analysis, PagePoint, project_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRange {
    Main(Range<usize>),
    File(PathBuf, Range<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub level: usize,
    pub number: Option<String>,
    pub title: String,
    pub source: Option<SourceRange>,
    pub point: Option<PagePoint>,
    pub page: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Heading,
    Figure,
    Table,
    Listing,
    Equation,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelTarget {
    pub name: String,
    pub kind: LabelKind,
    pub text: Option<String>,
    pub page: Option<String>,
}

impl Analysis {
    #[must_use]
    pub fn outline(&self) -> Vec<Heading> {
        let Some(document) = self.document() else {
            return Vec::new();
        };
        document
            .introspector()
            .query(&HeadingElem::ELEM.select())
            .iter()
            .filter_map(|element| {
                let heading = element.to_packed::<HeadingElem>()?;
                let point = point_of(document, element);
                Some(Heading {
                    level: heading.resolve_level(StyleChain::default()).get(),
                    number: heading.numbers.as_ref().map(ToString::to_string),
                    title: heading.body.plain_text().trim().to_owned(),
                    source: self.source_range(element.span()),
                    point,
                    page: point.and_then(|point| page_label(document, point.page)),
                })
            })
            .collect()
    }

    #[must_use]
    pub fn labels(&self) -> Vec<LabelTarget> {
        let Some(document) = self.document() else {
            return Vec::new();
        };
        let labelled = document.introspector().query_labelled();
        let mut elements = HashMap::new();
        for element in &labelled {
            if let Some(label) = element.label() {
                elements.entry(label).or_insert(element);
            }
        }
        let (labels, split) = typst_ide::analyze_labels(document);
        labels
            .into_iter()
            .take(split)
            .filter_map(|(label, text)| {
                let element = elements.get(&label)?;
                Some(LabelTarget {
                    name: label.resolve().to_string(),
                    kind: kind_of(element),
                    text: text.map(|text| text.to_string()),
                    page: point_of(document, element)
                        .and_then(|point| page_label(document, point.page)),
                })
            })
            .collect()
    }

    fn source_range(&self, span: Span) -> Option<SourceRange> {
        let id = span.id()?;
        let range = self.world().range(span)?;
        if id == self.world().main() {
            Some(SourceRange::Main(range))
        } else {
            Some(SourceRange::File(project_path(id)?, range))
        }
    }
}

fn point_of(document: &PagedDocument, element: &Content) -> Option<PagePoint> {
    let location = element.location()?;
    document
        .introspector()
        .position(location)
        .map(PagePoint::from)
}

fn page_label(document: &PagedDocument, index: usize) -> Option<String> {
    let page = document.pages().get(index)?;
    let printed = match &page.numbering {
        Some(Numbering::Pattern(pattern)) => pattern
            .apply(None, &[page.number])
            .ok()
            .map(|label| {
                label
                    .trim_matches(|character: char| !character.is_alphanumeric())
                    .to_owned()
            })
            .filter(|label| !label.is_empty()),
        Some(Numbering::Func(_)) => Some(page.number.to_string()),
        None => None,
    };
    Some(printed.unwrap_or_else(|| (index + 1).to_string()))
}

fn kind_of(element: &Content) -> LabelKind {
    if element.is::<HeadingElem>() {
        return LabelKind::Heading;
    }
    if element.is::<EquationElem>() {
        return LabelKind::Equation;
    }
    let Some(figure) = element.to_packed::<FigureElem>() else {
        return LabelKind::Other;
    };
    match figure.kind.get_ref(StyleChain::default()) {
        Smart::Custom(FigureKind::Elem(kind)) if *kind == TableElem::ELEM => LabelKind::Table,
        Smart::Custom(FigureKind::Elem(kind)) if *kind == RawElem::ELEM => LabelKind::Listing,
        _ => LabelKind::Figure,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::run;
    use crate::world::World;

    fn today() -> Option<Datetime> {
        Datetime::from_ymd(2026, 11, 20)
    }

    fn analysis(text: &str) -> Analysis {
        let world = Mutex::new(World::new(Arc::new(Fonts::embedded()), today()));
        run(&world, text, 1.0, false).analysis.unwrap()
    }

    #[test]
    fn the_outline_gives_the_headings_with_their_levels() {
        let headings = analysis("= One\n== Two\nText.\n= Three").outline();

        let found: Vec<(usize, &str)> = headings
            .iter()
            .map(|heading| (heading.level, heading.title.as_str()))
            .collect();
        assert_eq!(found, [(1, "One"), (2, "Two"), (1, "Three")]);
    }

    #[test]
    fn a_numbered_heading_gives_its_number() {
        let headings = analysis("#set heading(numbering: \"1.1\")\n= One\n== Two").outline();

        let numbers: Vec<Option<&str>> = headings
            .iter()
            .map(|heading| heading.number.as_deref())
            .collect();
        assert_eq!(numbers, [Some("1"), Some("1.1")]);
    }

    #[test]
    fn a_heading_without_numbering_has_no_number() {
        let headings = analysis("= One").outline();

        assert_eq!(headings[0].number, None);
    }

    #[test]
    fn a_heading_gives_its_range_in_the_main_file() {
        let text = "Text.\n\n= Start\nMore.";

        let headings = analysis(text).outline();

        let Some(SourceRange::Main(range)) = &headings[0].source else {
            panic!("{headings:?}");
        };
        assert_eq!(&text[range.clone()], "= Start");
    }

    #[test]
    fn a_heading_gives_its_page() {
        let headings = analysis("= One\n#pagebreak()\n= Two").outline();

        let pages: Vec<(Option<usize>, Option<&str>)> = headings
            .iter()
            .map(|heading| {
                (
                    heading.point.map(|point| point.page),
                    heading.page.as_deref(),
                )
            })
            .collect();
        assert_eq!(pages, [(Some(0), Some("1")), (Some(1), Some("2"))]);
    }

    #[test]
    fn a_page_with_roman_numbers_gives_the_printed_number() {
        let headings =
            analysis("#set page(numbering: \"i\")\n= One\n#pagebreak()\n= Two").outline();

        let pages: Vec<Option<&str>> = headings
            .iter()
            .map(|heading| heading.page.as_deref())
            .collect();
        assert_eq!(pages, [Some("i"), Some("ii")]);
    }

    #[test]
    fn a_heading_in_an_included_file_gives_the_file() {
        let folder = tempfile::tempdir().unwrap();
        let main = "= Main\n#include \"part.typ\"";
        fs::write(folder.path().join("main.typ"), main).unwrap();
        fs::write(folder.path().join("part.typ"), "= Part").unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            today(),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));

        let headings = run(&world, main, 1.0, false).analysis.unwrap().outline();

        assert_eq!(
            headings[1].source,
            Some(SourceRange::File(PathBuf::from("part.typ"), 0..6))
        );
    }

    #[test]
    fn a_document_with_an_error_has_no_outline() {
        assert_eq!(analysis("= One\n#let").outline(), []);
    }

    #[test]
    fn labels_give_the_kind_of_each_element() {
        let text = "= Start <start>\n\
            #figure(rect(), caption: [A box]) <box>\n\
            #figure(table(columns: 1)[A], caption: [Data]) <data>\n\
            #figure(raw(\"x\", block: true), caption: [Code]) <code>\n\
            $ x = 1 $ <sum>\n\
            #metadata(\"value\") <value>";

        let labels = analysis(text).labels();

        let kinds: Vec<(&str, LabelKind)> = labels
            .iter()
            .map(|label| (label.name.as_str(), label.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("start", LabelKind::Heading),
                ("box", LabelKind::Figure),
                ("data", LabelKind::Table),
                ("code", LabelKind::Listing),
                ("sum", LabelKind::Equation),
                ("value", LabelKind::Other),
            ]
        );
    }

    #[test]
    fn a_label_gives_the_text_and_the_page_of_its_element() {
        let text = "= Start <start>\n#pagebreak()\n#figure(rect(), caption: [A box]) <box>";

        let labels = analysis(text).labels();

        let found: Vec<(Option<&str>, Option<&str>)> = labels
            .iter()
            .map(|label| (label.text.as_deref(), label.page.as_deref()))
            .collect();
        assert_eq!(
            found,
            [(Some("Start"), Some("1")), (Some("A box"), Some("2"))]
        );
    }
}
