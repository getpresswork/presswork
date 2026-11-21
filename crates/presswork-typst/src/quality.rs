use presswork_project::{LengthUnit, TypesetSettings};
use typst::introspection::Tag;
use typst::layout::{Frame, FrameItem, Point, Transform};
use typst::model::ParElem;
use typst::syntax::Span;
use typst::text::TextItem;
use typst_layout::PagedDocument;

use crate::area::{Area, text_rect};
use crate::ide::Analysis;
use crate::preflight::{Problem, ProblemKind};

const NEW_LINE: f64 = 0.5;
const HYPHENS: [char; 3] = ['\u{ad}', '-', '\u{2010}'];
const GRID_TOLERANCE: f64 = 0.1;
const TIE: f64 = 1e-6;
const SIZE_TOLERANCE: f64 = 0.05;
const MICROMETERS_PER_POINT: f64 = 25_400.0 / 72.0;

#[derive(Debug, Clone, PartialEq)]
struct Line {
    page: usize,
    y: f64,
    size: f64,
    text: String,
    spacing: u32,
    span: Span,
    area: Area,
}

struct Finding {
    page: usize,
    kind: ProblemKind,
    span: Span,
    area: Area,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub start: f64,
    pub step: f64,
}

impl Grid {
    #[must_use]
    pub fn offset(&self, y: f64) -> f64 {
        let steps = ((y - self.start) / self.step - 0.5).ceil();
        let offset = y - (self.start + steps * self.step);
        if (offset + self.step / 2.0).abs() < TIE {
            offset + self.step
        } else {
            offset
        }
    }
}

impl Analysis {
    #[must_use]
    pub fn baseline_grid(&self, settings: &TypesetSettings) -> Option<Grid> {
        grid(&paragraphs(self.document()?), settings)
    }

    #[must_use]
    pub fn quality(&self, settings: &TypesetSettings) -> Vec<Problem> {
        let Some(document) = self.document() else {
            return Vec::new();
        };
        let paragraphs = paragraphs(document);
        let mut found = Vec::new();
        for lines in &paragraphs {
            check(lines, settings, &mut found);
        }
        if let (Some(step), Some(grid)) = (settings.baseline_grid, grid(&paragraphs, settings)) {
            check_grid(&paragraphs, grid, step.unit, &mut found);
        }
        found.sort_by_key(|finding| finding.page);
        found
            .into_iter()
            .map(|finding| Problem {
                page: Some(finding.page),
                kind: finding.kind,
                source: self.source_range(finding.span),
                area: Some(finding.area),
            })
            .collect()
    }
}

fn check(lines: &[Line], settings: &TypesetSettings, found: &mut Vec<Finding>) {
    let hyphen_limit = usize::try_from(settings.stacked_hyphens).unwrap_or(usize::MAX);
    let short_limit = usize::try_from(settings.short_last_line).unwrap_or(usize::MAX);
    let mut add = |line: &Line, kind, area| {
        found.push(Finding {
            page: line.page,
            kind,
            span: line.span,
            area,
        });
    };
    if settings.orphans
        && let [first, second, ..] = lines
        && moved(first, second)
    {
        add(first, ProblemKind::Orphan, first.area);
    }
    let mut run: Option<(usize, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        if line.text.trim_end().ends_with(HYPHENS) {
            let (start, count) = run.unwrap_or((index, 0));
            run = Some((start, count + 1));
            continue;
        }
        if let Some((start, count)) = run.take()
            && count > hyphen_limit
        {
            add(
                &lines[start],
                ProblemKind::StackedHyphens { lines: count },
                run_area(&lines[start..start + count]),
            );
        }
    }
    let Some((last, before)) = lines.split_last() else {
        return;
    };
    for line in before {
        if line.spacing > settings.loose_spacing {
            add(
                line,
                ProblemKind::LooseLine {
                    spacing: line.spacing,
                },
                line.area,
            );
        }
    }
    let Some(previous) = before.last() else {
        return;
    };
    if settings.widows && moved(previous, last) {
        add(last, ProblemKind::Widow, last.area);
    }
    let characters = last
        .text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count();
    if characters < short_limit {
        add(last, ProblemKind::ShortLastLine { characters }, last.area);
    }
}

fn body_size(paragraphs: &[Vec<Line>]) -> Option<f64> {
    let mut sizes: Vec<(f64, usize)> = Vec::new();
    for line in paragraphs.iter().flatten() {
        match sizes
            .iter_mut()
            .find(|(size, _)| (size - line.size).abs() < SIZE_TOLERANCE)
        {
            Some((_, count)) => *count += 1,
            None => sizes.push((line.size, 1)),
        }
    }
    sizes
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(size, _)| size)
}

fn body_lines(paragraphs: &[Vec<Line>]) -> impl Iterator<Item = &Line> {
    let size = body_size(paragraphs);
    paragraphs
        .iter()
        .flatten()
        .filter(move |line| size.is_some_and(|size| (line.size - size).abs() < SIZE_TOLERANCE))
}

fn grid(paragraphs: &[Vec<Line>], settings: &TypesetSettings) -> Option<Grid> {
    let step = settings.baseline_grid?.points();
    let start = match settings.baseline_start {
        Some(start) => start.points(),
        None => {
            body_lines(paragraphs)
                .map(|line| (line.page, line.y))
                .min_by(|first, second| first.0.cmp(&second.0).then(first.1.total_cmp(&second.1)))?
                .1
        }
    };
    Some(Grid { start, step })
}

fn check_grid(paragraphs: &[Vec<Line>], grid: Grid, unit: LengthUnit, found: &mut Vec<Finding>) {
    for line in body_lines(paragraphs) {
        let offset = grid.offset(line.y);
        if offset.abs() > GRID_TOLERANCE {
            found.push(Finding {
                page: line.page,
                kind: ProblemKind::OffGrid {
                    offset: micrometers(offset),
                    unit,
                },
                span: line.span,
                area: line.area,
            });
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "an offset is less than one grid step"
)]
fn micrometers(points: f64) -> i32 {
    (points * MICROMETERS_PER_POINT).round() as i32
}

fn run_area(lines: &[Line]) -> Area {
    let mut area = lines[0].area;
    for pair in lines.windows(2) {
        if moved(&pair[0], &pair[1]) {
            break;
        }
        area = area.union(pair[1].area);
    }
    area
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
        let area = Area::of(text_rect(text), Transform::translate(at.x, at.y));
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
                area,
            });
        }
        if let Some(line) = lines.last_mut() {
            line.text.push_str(&text.text);
            line.spacing = line.spacing.max(spacing(text));
            line.area = line.area.union(area);
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

    use presswork_project::{Length, LengthUnit};
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
            area: Area {
                x: 0.0,
                y: y - 8.0,
                width: 100.0,
                height: 10.0,
            },
        }
    }

    fn kinds(lines: &[Line]) -> Vec<(usize, ProblemKind)> {
        kinds_with(lines, &TypesetSettings::default())
    }

    fn kinds_with(lines: &[Line], settings: &TypesetSettings) -> Vec<(usize, ProblemKind)> {
        let mut found = Vec::new();
        check(lines, settings, &mut found);
        found
            .into_iter()
            .map(|finding| (finding.page, finding.kind))
            .collect()
    }

    #[test]
    fn the_limits_of_the_report_come_from_the_settings() {
        let short = [
            line(0, 20.0, "A paragraph that ends with"),
            line(0, 34.0, "it."),
        ];
        let mut wide = line(0, 20.0, "Wide spaces in this line");
        wide.spacing = 186;
        let loose = [wide, line(0, 34.0, "and the end of the paragraph.")];
        let hyphenated = |y| line(0, y, "hyphen\u{ad}");
        let hyphens = [
            line(0, 10.0, "Start"),
            hyphenated(24.0),
            hyphenated(38.0),
            hyphenated(52.0),
            line(0, 66.0, "and the end of the paragraph."),
        ];
        let settings = TypesetSettings {
            short_last_line: 3,
            loose_spacing: 200,
            stacked_hyphens: 2,
            ..TypesetSettings::default()
        };

        assert_eq!(kinds_with(&short, &settings), []);
        assert_eq!(kinds_with(&loose, &settings), []);
        assert_eq!(
            kinds_with(&hyphens, &settings),
            [(0, ProblemKind::StackedHyphens { lines: 3 })]
        );
    }

    #[test]
    fn widows_and_orphans_can_be_switched_off() {
        let orphan = [
            line(0, 500.0, "The first"),
            line(1, 20.0, "and the rest"),
            line(1, 34.0, "of it."),
        ];
        let widow = [
            line(0, 480.0, "The first line"),
            line(0, 494.0, "and a second line"),
            line(1, 20.0, "and the end."),
        ];
        let off = TypesetSettings {
            widows: false,
            orphans: false,
            ..TypesetSettings::default()
        };

        assert_eq!(kinds_with(&orphan, &off), []);
        assert_eq!(kinds_with(&widow, &off), []);
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

        let problems = analysis.quality(&TypesetSettings::default());

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

    const GRID: &str = "#set page(width: 200pt, height: auto, margin: 10pt)\n\
                        #set text(size: 10pt, top-edge: 10pt, bottom-edge: 0pt)\n\
                        #set par(leading: 2pt, spacing: 2pt)\n";

    fn grid_problems(text: &str, settings: &TypesetSettings) -> Vec<Problem> {
        let world = Mutex::new(World::new(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 21),
        ));
        analyze(&world, text)
            .analysis
            .unwrap()
            .quality(settings)
            .into_iter()
            .filter(|problem| matches!(problem.kind, ProblemKind::OffGrid { .. }))
            .collect()
    }

    fn twelve_points(start: Option<&str>) -> TypesetSettings {
        TypesetSettings {
            baseline_grid: Some(Length::parse("12pt").unwrap()),
            baseline_start: start.map(|start| Length::parse(start).unwrap()),
            ..TypesetSettings::default()
        }
    }

    #[test]
    fn body_lines_on_the_baseline_grid_have_no_problem() {
        let text = format!(
            "{GRID}The first paragraph has enough words to fill two lines here.\n\nThe second one."
        );

        assert_eq!(grid_problems(&text, &twelve_points(Some("20pt"))), []);
        assert_eq!(grid_problems(&text, &TypesetSettings::default()), []);
    }

    #[test]
    fn a_body_line_off_the_baseline_grid_is_a_problem() {
        let text = format!("{GRID}The first one.\n\nThe second one.\n\n#v(5pt)\nThe third one.");

        let found = grid_problems(&text, &twelve_points(Some("20pt")));

        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            matches!(found[0].kind, ProblemKind::OffGrid { offset, unit: LengthUnit::Pt } if offset != 0),
            "{found:?}"
        );
        assert!(found[0].area.is_some_and(|area| area.y > 40.0), "{found:?}");
    }

    #[test]
    fn without_a_start_the_first_body_line_sets_the_baseline_grid() {
        let text = format!("{GRID}#v(3pt)\nThe first one.\n\nThe second one.\n\nThe third one.");

        assert_eq!(grid_problems(&text, &twelve_points(None)), []);
        assert_eq!(grid_problems(&text, &twelve_points(Some("20pt"))).len(), 3);
    }

    #[test]
    fn lines_in_a_different_size_do_not_count_for_the_baseline_grid() {
        let text = format!(
            "{GRID}The first one.\n\nThe second one.\n\nThe third one.\n\n\
             #v(3pt)\n#text(size: 8pt)[A small note.]"
        );

        assert_eq!(grid_problems(&text, &twelve_points(Some("20pt"))), []);
    }

    #[test]
    fn a_running_head_does_not_set_the_baseline_grid() {
        let text = format!(
            "{GRID}#set page(height: 200pt, margin: (top: 40pt, rest: 10pt), header: [Running head])\n\
             The first one.\n\nThe second one."
        );

        assert_eq!(grid_problems(&text, &twelve_points(None)), []);
    }

    #[test]
    fn the_offset_goes_to_the_nearest_grid_line() {
        let grid = Grid {
            start: 10.0,
            step: 12.0,
        };

        assert!(grid.offset(22.0).abs() < 1e-9);
        assert!((grid.offset(27.0) - 5.0).abs() < 1e-9);
        assert!((grid.offset(33.0) + 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_line_in_the_middle_between_two_grid_lines_is_below_the_upper_one() {
        let grid = Grid {
            start: 10.0,
            step: 12.0,
        };

        assert!((grid.offset(28.0) - 6.0).abs() < 1e-9);
        assert!((grid.offset(4.0) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn each_problem_has_the_area_of_its_line() {
        let world = Mutex::new(World::new(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 21),
        ));
        let analysis = analyze(&world, &format!("{NARROW}A line of text \\ it."))
            .analysis
            .unwrap();

        let problems = analysis.quality(&TypesetSettings::default());

        let area = problems[0].area.unwrap();
        assert!((area.x - 10.0).abs() < 0.5, "{area:?}");
        assert!(area.width > 5.0 && area.width < 20.0, "{area:?}");
        assert!(area.y > 15.0 && area.y < 30.0, "{area:?}");
        assert!(area.height > 9.0 && area.height < 15.0, "{area:?}");
    }

    #[test]
    fn stacked_hyphens_have_the_area_of_all_their_lines() {
        let hyphenated = |y| line(0, y, "hyphen\u{ad}");
        let lines = [
            line(0, 10.0, "Start"),
            hyphenated(24.0),
            hyphenated(38.0),
            hyphenated(52.0),
            hyphenated(66.0),
            line(0, 80.0, "and the end of the paragraph."),
        ];
        let mut found = Vec::new();

        check(&lines, &TypesetSettings::default(), &mut found);

        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].area,
            Area {
                x: 0.0,
                y: 16.0,
                width: 100.0,
                height: 52.0
            }
        );
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
