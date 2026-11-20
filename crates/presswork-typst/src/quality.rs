use typst::introspection::Tag;
use typst::layout::{Frame, FrameItem, Point};
use typst::model::ParElem;
use typst::syntax::Span;
use typst::text::TextItem;
use typst_layout::PagedDocument;

use crate::ide::Analysis;
use crate::preflight::{Problem, ProblemKind};

const NEW_LINE: f64 = 0.5;
const HYPHENS: [char; 3] = ['\u{ad}', '-', '\u{2010}'];
const HYPHEN_RUN: usize = 3;
const SHORT_LINE: usize = 5;
const LOOSE_SPACING: u32 = 150;

#[derive(Debug, Clone, PartialEq)]
struct Line {
    page: usize,
    y: f64,
    size: f64,
    text: String,
    spacing: u32,
    span: Span,
}

struct Finding {
    page: usize,
    kind: ProblemKind,
    span: Span,
}

impl Analysis {
    #[must_use]
    pub fn quality(&self) -> Vec<Problem> {
        let Some(document) = self.document() else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for lines in paragraphs(document) {
            check(&lines, &mut found);
        }
        found.sort_by_key(|finding| finding.page);
        found
            .into_iter()
            .map(|finding| Problem {
                page: Some(finding.page),
                kind: finding.kind,
                source: self.source_range(finding.span),
            })
            .collect()
    }
}

fn check(lines: &[Line], found: &mut Vec<Finding>) {
    let mut add = |line: &Line, kind| {
        found.push(Finding {
            page: line.page,
            kind,
            span: line.span,
        });
    };
    if let [first, second, ..] = lines
        && moved(first, second)
    {
        add(first, ProblemKind::Orphan);
    }
    let mut run: Option<(usize, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        if line.text.trim_end().ends_with(HYPHENS) {
            let (start, count) = run.unwrap_or((index, 0));
            run = Some((start, count + 1));
            continue;
        }
        if let Some((start, count)) = run.take()
            && count > HYPHEN_RUN
        {
            add(&lines[start], ProblemKind::StackedHyphens { lines: count });
        }
    }
    let Some((last, before)) = lines.split_last() else {
        return;
    };
    for line in before {
        if line.spacing > LOOSE_SPACING {
            add(
                line,
                ProblemKind::LooseLine {
                    spacing: line.spacing,
                },
            );
        }
    }
    let Some(previous) = before.last() else {
        return;
    };
    if moved(previous, last) {
        add(last, ProblemKind::Widow);
    }
    let characters = last
        .text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    if characters < SHORT_LINE {
        add(last, ProblemKind::ShortLastLine { characters });
    }
}

fn moved(before: &Line, after: &Line) -> bool {
    after.page != before.page || after.y < before.y
}

#[derive(Default)]
struct Paragraphs {
    lines: Vec<Vec<Line>>,
    open: Vec<(typst::introspection::Location, usize)>,
}

fn paragraphs(document: &PagedDocument) -> Vec<Vec<Line>> {
    let mut paragraphs = Paragraphs::default();
    for (page, content) in document.pages().iter().enumerate() {
        paragraphs.frame(&content.frame, Point::zero(), page);
    }
    paragraphs.lines
}

impl Paragraphs {
    fn frame(&mut self, frame: &Frame, offset: Point, page: usize) {
        for (position, item) in frame.items() {
            let at = offset + *position;
            match item {
                FrameItem::Group(group) => {
                    let shift = Point::new(group.transform.tx, group.transform.ty);
                    self.frame(&group.frame, at + shift, page);
                }
                FrameItem::Tag(Tag::Start(content, _)) if content.is::<ParElem>() => {
                    if let Some(location) = content.location() {
                        self.open.push((location, self.lines.len()));
                        self.lines.push(Vec::new());
                    }
                }
                FrameItem::Tag(Tag::End(location, ..)) => {
                    if self.open.last().is_some_and(|(open, _)| open == location) {
                        self.open.pop();
                    }
                }
                FrameItem::Text(text) => self.text(text, at, page),
                FrameItem::Tag(_)
                | FrameItem::Shape(..)
                | FrameItem::Image(..)
                | FrameItem::Link(..) => {}
            }
        }
    }

    fn text(&mut self, text: &TextItem, at: Point, page: usize) {
        let Some(&(_, index)) = self.open.last() else {
            return;
        };
        let lines = &mut self.lines[index];
        let y = at.y.to_pt();
        let continues = lines
            .last()
            .is_some_and(|line| line.page == page && (line.y - y).abs() <= NEW_LINE * line.size);
        if !continues {
            lines.push(Line {
                page,
                y,
                size: text.size.to_pt(),
                text: String::new(),
                spacing: 100,
                span: text
                    .glyphs
                    .first()
                    .map_or_else(Span::detached, |glyph| glyph.span.0),
            });
        }
        if let Some(line) = lines.last_mut() {
            line.text.push_str(&text.text);
            line.spacing = line.spacing.max(spacing(text));
        }
    }
}

fn spacing(text: &TextItem) -> u32 {
    text.glyphs
        .iter()
        .filter(|glyph| {
            let range = usize::from(glyph.range.start)..usize::from(glyph.range.end);
            text.text.get(range) == Some(" ")
        })
        .filter_map(|glyph| {
            let natural = text.font.x_advance(glyph.id)?.get();
            (natural > 0.0).then(|| percent(glyph.x_advance.get() / natural))
        })
        .max()
        .unwrap_or(100)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a spacing ratio is a small positive number"
)]
fn percent(ratio: f64) -> u32 {
    (ratio * 100.0).round().clamp(0.0, f64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::analyze;
    use crate::world::World;

    fn layout(text: &str) -> Vec<Vec<Line>> {
        let world = Mutex::new(World::new(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
        ));
        let analysis = analyze(&world, text).analysis.unwrap();
        paragraphs(analysis.document().unwrap())
    }

    fn line(page: usize, y: f64, text: &str) -> Line {
        Line {
            page,
            y,
            size: 10.0,
            text: text.to_owned(),
            spacing: 100,
            span: Span::detached(),
        }
    }

    fn kinds(lines: &[Line]) -> Vec<(usize, ProblemKind)> {
        let mut found = Vec::new();
        check(lines, &mut found);
        found
            .into_iter()
            .map(|finding| (finding.page, finding.kind))
            .collect()
    }

    const NARROW: &str =
        "#set page(width: 120pt, height: auto, margin: 10pt)\n#set text(size: 10pt)\n";

    #[test]
    fn each_paragraph_gives_its_lines_in_order() {
        let paragraphs = layout(&format!(
            "{NARROW}= A heading\n\nThe first paragraph has enough words for two or three lines.\n\nShort one."
        ));

        assert_eq!(paragraphs.len(), 2);
        assert!(paragraphs[0].len() >= 2, "{paragraphs:?}");
        assert!(paragraphs[0].windows(2).all(|pair| pair[0].y < pair[1].y));
        assert_eq!(paragraphs[1].len(), 1);
        assert_eq!(paragraphs[1][0].text, "Short one.");
    }

    #[test]
    fn a_paragraph_can_continue_on_the_next_page() {
        let paragraphs = layout(
            "#set page(width: 120pt, height: 60pt, margin: 5pt)\n#set text(size: 10pt, costs: (widow: 0%, orphan: 0%))\n\
             #lorem(40)",
        );

        assert_eq!(paragraphs.len(), 1);
        let pages: Vec<usize> = paragraphs[0].iter().map(|line| line.page).collect();
        assert_eq!(pages.first(), Some(&0));
        assert!(pages.last() > Some(&0), "{pages:?}");
    }

    #[test]
    fn a_hyphenated_line_ends_with_a_soft_hyphen() {
        let paragraphs = layout(&format!(
            "{NARROW}#set par(justify: true)\n#set text(lang: \"en\", hyphenate: true)\n\
             Extraordinarily comprehensive internationalization considerations."
        ));

        assert!(
            paragraphs[0]
                .iter()
                .any(|line| line.text.ends_with('\u{ad}')),
            "{paragraphs:?}"
        );
    }

    #[test]
    fn justified_lines_give_the_width_of_their_word_spaces() {
        let paragraphs = layout(&format!(
            "{NARROW}#set par(justify: true)\n#set text(hyphenate: false)\n\
             A typesetter considers incomprehensibilities thoroughly."
        ));

        let lines = &paragraphs[0];
        assert!(
            lines[..lines.len() - 1]
                .iter()
                .any(|line| line.spacing > 150),
            "{lines:?}"
        );
        assert_eq!(lines.last().map(|line| line.spacing), Some(100));
    }

    #[test]
    fn the_report_gives_the_page_and_the_source_of_each_problem() {
        let world = Mutex::new(World::new(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
        ));
        let analysis = analyze(&world, "A line of text \\ it.").analysis.unwrap();

        let problems = analysis.quality();

        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].page, Some(0));
        assert_eq!(
            problems[0].kind,
            ProblemKind::ShortLastLine { characters: 3 }
        );
        assert!(matches!(
            problems[0].source,
            Some(crate::outline::SourceRange::Main(_))
        ));
    }

    #[test]
    fn a_first_line_alone_before_a_break_is_an_orphan() {
        let lines = [
            line(0, 500.0, "The first"),
            line(1, 20.0, "and the rest"),
            line(1, 34.0, "of it."),
        ];

        assert_eq!(kinds(&lines), [(0, ProblemKind::Orphan)]);
    }

    #[test]
    fn a_last_line_alone_after_a_break_is_a_widow() {
        let lines = [
            line(0, 480.0, "The first line"),
            line(0, 494.0, "and a second line"),
            line(1, 20.0, "and the end."),
        ];

        assert_eq!(kinds(&lines), [(1, ProblemKind::Widow)]);
    }

    #[test]
    fn a_new_column_is_a_break() {
        let lines = [
            line(0, 480.0, "The first line"),
            line(0, 494.0, "and a second line"),
            line(0, 20.0, "and the end."),
        ];

        assert_eq!(kinds(&lines), [(0, ProblemKind::Widow)]);
    }

    #[test]
    fn a_short_last_line_is_a_problem() {
        let lines = [
            line(0, 20.0, "A paragraph that ends with"),
            line(0, 34.0, "it."),
        ];

        assert_eq!(
            kinds(&lines),
            [(0, ProblemKind::ShortLastLine { characters: 3 })]
        );
        assert_eq!(kinds(&[line(0, 20.0, "Short.")]), []);
    }

    #[test]
    fn more_than_three_hyphens_in_sequence_are_a_problem() {
        let hyphenated = |y| line(0, y, "hyphen\u{ad}");
        let mut lines = vec![
            line(0, 10.0, "Start"),
            hyphenated(24.0),
            hyphenated(38.0),
            hyphenated(52.0),
            line(0, 66.0, "and the end of the paragraph."),
        ];
        assert_eq!(kinds(&lines), []);

        lines.insert(4, hyphenated(60.0));
        assert_eq!(
            kinds(&lines),
            [(0, ProblemKind::StackedHyphens { lines: 4 })]
        );
    }

    #[test]
    fn a_line_with_wide_word_spaces_is_loose() {
        let mut first = line(0, 20.0, "Wide spaces in this line");
        first.spacing = 186;
        let mut last = line(0, 34.0, "and the end of the paragraph.");
        last.spacing = 300;

        assert_eq!(
            kinds(&[first, last]),
            [(0, ProblemKind::LooseLine { spacing: 186 })]
        );
    }
}
