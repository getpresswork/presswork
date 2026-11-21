use std::collections::HashSet;

use presswork_project::{Colors, ExportProfile, PageSize};
use serde::Serialize;
use typst::foundations::Repr as _;
use typst::layout::{Abs, Frame, FrameItem, Point, Rect, Size, Transform};
use typst::syntax::Span;
use typst::text::TextItem;
use typst::visualize::{Color, ColorSpace, Image, ImageKind, Paint, ProcessColorSpace};

use crate::area::{Area, glyph_rect, text_rect};
use crate::ide::Analysis;
use crate::job::Output;
use crate::outline::SourceRange;

const POINTS_PER_INCH: f64 = 72.0;
const MICROMETERS_PER_POINT: f64 = 25_400.0 / POINTS_PER_INCH;
const PAGE_SIZE_TOLERANCE: u32 = 100;
const MISSING_GLYPHS_SHOWN: usize = 10;
const UNKNOWN_FAMILY: &str = "unknown font family: ";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rules {
    pub minimum_ppi: Option<u32>,
    pub page_size: Option<PageSize>,
    pub colors: Option<Colors>,
}

impl Rules {
    #[must_use]
    pub fn of(profile: &ExportProfile) -> Self {
        Self {
            minimum_ppi: profile.image_ppi,
            page_size: profile.page_size,
            colors: profile.colors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProblemKind {
    LowResolution {
        ppi: u32,
        minimum: u32,
    },
    MissingAltText,
    MissingFont {
        family: String,
    },
    MissingGlyphs {
        characters: String,
    },
    PageSize {
        #[serde(rename = "width-um")]
        width: u32,
        #[serde(rename = "height-um")]
        height: u32,
        needed: PageSize,
    },
    Color {
        color: String,
        permitted: Colors,
    },
    GradientSpace {
        space: String,
        permitted: Colors,
    },
    Widow,
    Orphan,
    StackedHyphens {
        lines: usize,
    },
    ShortLastLine {
        characters: usize,
    },
    LooseLine {
        spacing: u32,
    },
}

impl ProblemKind {
    #[must_use]
    pub fn depends_on_profile(&self) -> bool {
        matches!(
            self,
            Self::LowResolution { .. }
                | Self::PageSize { .. }
                | Self::Color { .. }
                | Self::GradientSpace { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub page: Option<usize>,
    pub kind: ProblemKind,
    pub source: Option<SourceRange>,
    pub area: Option<Area>,
}

impl Output {
    #[must_use]
    pub fn preflight_profiles<'a>(
        &self,
        profiles: &'a [ExportProfile],
    ) -> Vec<(&'a ExportProfile, Problem)> {
        let mut found: Vec<(&ExportProfile, Problem)> = Vec::new();
        for profile in profiles {
            for problem in self.preflight(&Rules::of(profile)) {
                if problem.kind.depends_on_profile()
                    || !found.iter().any(|(_, known)| *known == problem)
                {
                    found.push((profile, problem));
                }
            }
        }
        found
    }

    #[must_use]
    pub fn preflight(&self, rules: &Rules) -> Vec<Problem> {
        let mut families = HashSet::new();
        let mut problems: Vec<Problem> = self
            .diagnostics
            .iter()
            .filter_map(|diagnostic| {
                let family = diagnostic.message.strip_prefix(UNKNOWN_FAMILY)?;
                families.insert(family).then(|| Problem {
                    page: None,
                    kind: ProblemKind::MissingFont {
                        family: family.to_owned(),
                    },
                    source: diagnostic.source.clone(),
                    area: None,
                })
            })
            .collect();
        if let Some(analysis) = &self.analysis {
            problems.extend(analysis.preflight(rules));
        }
        problems
    }
}

impl Analysis {
    #[must_use]
    pub fn preflight(&self, rules: &Rules) -> Vec<Problem> {
        let mut check = Check {
            analysis: self,
            rules,
            page: 0,
            problems: Vec::new(),
            colors: HashSet::new(),
            spaces: HashSet::new(),
            missing: Vec::new(),
            missing_source: None,
            missing_area: None,
            inside: None,
        };
        if let Some(document) = self.document() {
            for (page, content) in document.pages().iter().enumerate() {
                check.page = page;
                check.page_size(&content.frame);
                check.frame(&content.frame, Transform::identity());
                check.missing_glyphs();
            }
        }
        check.problems
    }
}

struct Check<'a> {
    analysis: &'a Analysis,
    rules: &'a Rules,
    page: usize,
    problems: Vec<Problem>,
    colors: HashSet<String>,
    spaces: HashSet<String>,
    missing: Vec<char>,
    missing_source: Option<SourceRange>,
    missing_area: Option<Area>,
    inside: Option<Area>,
}

impl Check<'_> {
    fn push(&mut self, kind: ProblemKind, source: Option<SourceRange>, area: Option<Area>) {
        self.problems.push(Problem {
            page: Some(self.page),
            kind,
            source,
            area: self.inside.or(area),
        });
    }

    fn page_size(&mut self, frame: &Frame) {
        let Some(needed) = self.rules.page_size else {
            return;
        };
        let width = micrometers(frame.width());
        let height = micrometers(frame.height());
        if width.abs_diff(needed.width) > PAGE_SIZE_TOLERANCE
            || height.abs_diff(needed.height) > PAGE_SIZE_TOLERANCE
        {
            self.push(
                ProblemKind::PageSize {
                    width,
                    height,
                    needed,
                },
                None,
                None,
            );
        }
    }

    fn frame(&mut self, frame: &Frame, transform: Transform) {
        for (position, item) in frame.items() {
            let transform = transform.pre_concat(Transform::translate(position.x, position.y));
            match item {
                FrameItem::Group(group) => {
                    self.frame(&group.frame, transform.pre_concat(group.transform));
                }
                FrameItem::Text(text) => self.text(text, transform),
                FrameItem::Shape(shape, span) => {
                    let area = Area::of(shape.bbox(false), transform);
                    if let Some(fill) = &shape.fill {
                        self.paint(fill, *span, transform, area);
                    }
                    if let Some(stroke) = &shape.stroke {
                        self.paint(&stroke.paint, *span, transform, area);
                    }
                }
                FrameItem::Image(image, size, span) => {
                    self.image(image, *size, *span, transform);
                }
                FrameItem::Link(..) | FrameItem::Tag(_) => {}
            }
        }
    }

    fn text(&mut self, text: &TextItem, transform: Transform) {
        let span = text
            .glyphs
            .first()
            .map_or_else(Span::detached, |glyph| glyph.span.0);
        let area = Area::of(text_rect(text), transform);
        self.paint(&text.fill, span, transform, area);
        if let Some(stroke) = &text.stroke {
            self.paint(&stroke.paint, span, transform, area);
        }
        for (index, glyph) in text
            .glyphs
            .iter()
            .enumerate()
            .filter(|(_, glyph)| glyph.id == 0)
        {
            let range = usize::from(glyph.range.start)..usize::from(glyph.range.end);
            let characters = text.text.get(range).unwrap_or_default();
            for character in characters
                .chars()
                .filter(|character| !character.is_whitespace())
            {
                if self.missing_source.is_none() {
                    self.missing_source = self.analysis.source_range(glyph.span.0);
                }
                if self.missing_area.is_none() {
                    self.missing_area = Some(
                        self.inside
                            .unwrap_or_else(|| Area::of(glyph_rect(text, index), transform)),
                    );
                }
                if !self.missing.contains(&character) {
                    self.missing.push(character);
                }
            }
        }
    }

    fn missing_glyphs(&mut self) {
        if self.missing.is_empty() {
            return;
        }
        let characters = self
            .missing
            .drain(..)
            .take(MISSING_GLYPHS_SHOWN)
            .map(String::from)
            .collect::<Vec<_>>()
            .join(" ");
        let source = self.missing_source.take();
        let area = self.missing_area.take();
        self.push(ProblemKind::MissingGlyphs { characters }, source, area);
    }

    fn paint(&mut self, paint: &Paint, span: Span, transform: Transform, area: Area) {
        let Some(permitted) = self.rules.colors else {
            return;
        };
        match paint {
            Paint::Solid(color) => self.color(color, permitted, span, area),
            Paint::Gradient(gradient) => {
                let space = gradient.space();
                if !permits(permitted, &space) {
                    let space = space_name(&space).to_owned();
                    if self.spaces.insert(space.clone()) {
                        let source = self.analysis.source_range(span);
                        self.push(
                            ProblemKind::GradientSpace { space, permitted },
                            source,
                            Some(area),
                        );
                    }
                }
            }
            Paint::Tiling(tiling) => {
                let outer = self.inside;
                self.inside = Some(outer.unwrap_or(area));
                self.frame(tiling.frame(), transform);
                self.inside = outer;
            }
        }
    }

    fn color(&mut self, color: &Color, permitted: Colors, span: Span, area: Area) {
        if permits(permitted, &color.space()) {
            return;
        }
        let color = color.repr().to_string();
        if self.colors.insert(color.clone()) {
            let source = self.analysis.source_range(span);
            self.push(ProblemKind::Color { color, permitted }, source, Some(area));
        }
    }

    fn image(&mut self, image: &Image, size: Size, span: Span, transform: Transform) {
        let source = self.analysis.source_range(span);
        let area = Some(Area::of(
            Rect::from_pos_size(Point::zero(), size),
            transform,
        ));
        if image.alt().is_none() {
            self.push(ProblemKind::MissingAltText, source.clone(), area);
        }
        if let (Some(minimum), ImageKind::Raster(raster)) = (self.rules.minimum_ppi, image.kind()) {
            let scale_x = transform.sx.get().hypot(transform.ky.get());
            let scale_y = transform.kx.get().hypot(transform.sy.get());
            let across = f64::from(raster.width()) / (size.x.to_pt() * scale_x / POINTS_PER_INCH);
            let down = f64::from(raster.height()) / (size.y.to_pt() * scale_y / POINTS_PER_INCH);
            let ppi = whole(across.min(down));
            if ppi < minimum {
                self.push(ProblemKind::LowResolution { ppi, minimum }, source, area);
            }
        }
    }
}

fn permits(permitted: Colors, space: &ColorSpace) -> bool {
    matches!(
        (permitted, space),
        (_, ColorSpace::Process(ProcessColorSpace::D65Gray))
            | (
                Colors::Cmyk,
                ColorSpace::Process(ProcessColorSpace::Cmyk) | ColorSpace::Spot(_)
            )
    )
}

fn space_name(space: &ColorSpace) -> &'static str {
    match space {
        ColorSpace::Process(ProcessColorSpace::D65Gray) => "luma",
        ColorSpace::Process(ProcessColorSpace::Oklab) => "oklab",
        ColorSpace::Process(ProcessColorSpace::Oklch) => "oklch",
        ColorSpace::Process(ProcessColorSpace::LinearRgb) => "color.linear-rgb",
        ColorSpace::Process(ProcessColorSpace::Srgb) => "rgb",
        ColorSpace::Process(ProcessColorSpace::Cmyk) => "cmyk",
        ColorSpace::Process(ProcessColorSpace::Hsl) => "color.hsl",
        ColorSpace::Process(ProcessColorSpace::Hsv) => "color.hsv",
        ColorSpace::Spot(_) => "spot",
    }
}

fn micrometers(length: Abs) -> u32 {
    whole(length.to_pt() * MICROMETERS_PER_POINT)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a resolution is a small positive number"
)]
fn whole(value: f64) -> u32 {
    value.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use presswork_project::{ExportFormat, LengthUnit};
    use typst::foundations::Datetime;

    use super::*;
    use crate::fonts::Fonts;
    use crate::job::run;
    use crate::world::World;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let pixels = usize::try_from(width * height * 3).unwrap();
        writer.write_image_data(&vec![200; pixels]).unwrap();
        writer.finish().unwrap();
        bytes
    }

    fn output(text: &str) -> (Output, tempfile::TempDir) {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("main.typ"), text).unwrap();
        fs::write(folder.path().join("dot.png"), png(100, 100)).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        (run(&world, text, 1.0, false), folder)
    }

    fn problems(text: &str, rules: Rules) -> Vec<Problem> {
        output(text).0.preflight(&rules)
    }

    #[test]
    fn each_profile_gives_its_problems_and_the_document_problems_show_once() {
        let print = ExportProfile {
            image_ppi: Some(300),
            colors: Some(Colors::Cmyk),
            ..ExportProfile::named("Print", ExportFormat::Pdf)
        };
        let proof = ExportProfile {
            image_ppi: Some(150),
            ..ExportProfile::named("Proof", ExportFormat::Pdf)
        };
        let profiles = [print, proof];
        let (output, _folder) =
            output("#image(\"dot.png\", width: 1in)\n#text(fill: rgb(\"#ff0000\"))[Red]");

        let found = output.preflight_profiles(&profiles);

        let names: Vec<(&str, ProblemKind)> = found
            .iter()
            .map(|(profile, problem)| (profile.name.as_str(), problem.kind.clone()))
            .collect();
        assert_eq!(
            names,
            [
                ("Print", ProblemKind::MissingAltText),
                (
                    "Print",
                    ProblemKind::LowResolution {
                        ppi: 100,
                        minimum: 300
                    }
                ),
                (
                    "Print",
                    ProblemKind::Color {
                        color: "rgb(\"#ff0000\")".to_owned(),
                        permitted: Colors::Cmyk
                    }
                ),
                (
                    "Proof",
                    ProblemKind::LowResolution {
                        ppi: 100,
                        minimum: 150
                    }
                ),
            ]
        );
        assert!(!ProblemKind::MissingAltText.depends_on_profile());
        assert!(
            ProblemKind::LowResolution {
                ppi: 100,
                minimum: 150
            }
            .depends_on_profile()
        );
    }

    fn kinds(problems: &[Problem]) -> Vec<(Option<usize>, ProblemKind)> {
        problems
            .iter()
            .map(|problem| (problem.page, problem.kind.clone()))
            .collect()
    }

    fn resolution(minimum: u32) -> Rules {
        Rules {
            minimum_ppi: Some(minimum),
            ..Rules::default()
        }
    }

    fn colors(permitted: Colors) -> Rules {
        Rules {
            colors: Some(permitted),
            ..Rules::default()
        }
    }

    const MARGIN: &str = "#set page(margin: 10pt)\n";

    fn assert_area(area: Option<Area>, expected: [f64; 4]) {
        let area = area.expect("an area");
        let actual = [area.x, area.y, area.width, area.height];
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| (actual - expected).abs() < 0.5),
            "{area:?}"
        );
    }

    #[test]
    fn an_image_problem_has_the_area_of_the_image() {
        let found = problems(
            &format!("{MARGIN}#image(\"dot.png\", width: 1in)"),
            Rules::default(),
        );

        assert_eq!(kinds(&found), [(Some(0), ProblemKind::MissingAltText)]);
        assert_area(found[0].area, [10.0, 10.0, 72.0, 72.0]);
    }

    #[test]
    fn the_area_of_a_scaled_image_counts_the_scale() {
        let found = problems(
            &format!("{MARGIN}#scale(50%)[#image(\"dot.png\", width: 1in)]"),
            Rules::default(),
        );

        assert_area(found[0].area, [28.0, 28.0, 36.0, 36.0]);
    }

    #[test]
    fn a_color_problem_has_the_area_of_its_first_use() {
        let found = problems(
            &format!(
                "{MARGIN}#rect(width: 50pt, height: 20pt, fill: rgb(\"#ff0000\"))\n\
                 #rect(fill: rgb(\"#ff0000\"))"
            ),
            colors(Colors::Cmyk),
        );

        assert_eq!(found.len(), 1, "{found:?}");
        assert_area(found[0].area, [10.0, 10.0, 50.0, 20.0]);
    }

    #[test]
    fn a_color_in_a_tiling_has_the_area_of_the_filled_shape() {
        let tiling = "tiling(size: (5pt, 5pt))[#square(size: 2pt, fill: rgb(\"#ff0000\"))]";

        let found = problems(
            &format!("{MARGIN}#rect(width: 50pt, height: 20pt, fill: {tiling})"),
            colors(Colors::Cmyk),
        );

        assert_eq!(found.len(), 1, "{found:?}");
        assert_area(found[0].area, [10.0, 10.0, 50.0, 20.0]);
    }

    #[test]
    fn missing_glyphs_have_the_area_of_the_first_one() {
        let found = problems(&format!("{MARGIN}日 and 本"), Rules::default());

        let area = found[0].area.unwrap();
        assert!((area.x - 10.0).abs() < 0.5, "{area:?}");
        assert!(area.width > 1.0 && area.width < 12.0, "{area:?}");
    }

    #[test]
    fn a_small_image_at_a_large_size_has_a_low_resolution() {
        let found = problems(
            "#image(\"dot.png\", width: 1in, alt: \"A dot\")",
            resolution(300),
        );

        assert_eq!(
            kinds(&found),
            [(
                Some(0),
                ProblemKind::LowResolution {
                    ppi: 100,
                    minimum: 300
                }
            )]
        );
        assert!(matches!(found[0].source, Some(SourceRange::Main(_))));
    }

    #[test]
    fn an_image_with_enough_pixels_passes() {
        let found = problems(
            "#image(\"dot.png\", width: 0.25in, alt: \"A dot\")",
            resolution(300),
        );

        assert_eq!(found, []);
    }

    #[test]
    fn a_scaled_image_counts_its_scale() {
        let found = problems(
            "#scale(50%)[#image(\"dot.png\", width: 1in, alt: \"A dot\")]",
            resolution(300),
        );

        assert_eq!(
            kinds(&found),
            [(
                Some(0),
                ProblemKind::LowResolution {
                    ppi: 200,
                    minimum: 300
                }
            )]
        );
    }

    #[test]
    fn an_image_without_alternative_text_is_a_problem() {
        let found = problems(
            "Text.\n#pagebreak()\n#image(\"dot.png\", width: 1in)",
            Rules::default(),
        );

        assert_eq!(kinds(&found), [(Some(1), ProblemKind::MissingAltText)]);
    }

    #[test]
    fn a_font_family_that_is_not_available_is_a_problem_once() {
        let found = problems(
            "#set text(font: \"No Such Font\")\nOne.\n#text(font: \"No Such Font\")[Two.]",
            Rules::default(),
        );

        assert_eq!(
            kinds(&found),
            [(
                None,
                ProblemKind::MissingFont {
                    family: "no such font".to_owned()
                }
            )]
        );
        assert!(matches!(found[0].source, Some(SourceRange::Main(_))));
        assert_eq!(found[0].area, None);
    }

    #[test]
    fn characters_without_a_glyph_are_a_problem_for_each_page() {
        let found = problems(
            "Text 日本 and 日.\n#pagebreak()\nMore 語.",
            Rules::default(),
        );

        assert_eq!(
            kinds(&found),
            [
                (
                    Some(0),
                    ProblemKind::MissingGlyphs {
                        characters: "日 本".to_owned()
                    }
                ),
                (
                    Some(1),
                    ProblemKind::MissingGlyphs {
                        characters: "語".to_owned()
                    }
                )
            ]
        );
    }

    #[test]
    fn a_page_with_a_different_size_is_a_problem() {
        let needed = PageSize::parse("148mm x 210mm").unwrap();
        let rules = Rules {
            page_size: Some(needed),
            ..Rules::default()
        };

        let found = problems(
            "#set page(width: 148mm, height: 210mm)\nA5.\n#pagebreak()\n#set page(width: 100mm, height: 150mm)\nSmall.",
            rules,
        );

        assert_eq!(
            kinds(&found),
            [(
                Some(1),
                ProblemKind::PageSize {
                    width: 100_000,
                    height: 150_000,
                    needed
                }
            )]
        );
        assert_eq!(found[0].area, None);
        assert_eq!(needed.unit, LengthUnit::Mm);
    }

    #[test]
    fn a_color_outside_the_permitted_colors_is_a_problem_once() {
        let text = "#text(fill: rgb(\"#ff0000\"))[Red] #text(fill: cmyk(0%, 100%, 100%, 0%))[Red] \
                    #text(fill: luma(40%))[Gray] #rect(fill: rgb(\"#ff0000\"))";

        assert_eq!(
            kinds(&problems(text, colors(Colors::Cmyk))),
            [(
                Some(0),
                ProblemKind::Color {
                    color: "rgb(\"#ff0000\")".to_owned(),
                    permitted: Colors::Cmyk
                }
            )]
        );
        assert_eq!(problems(text, Rules::default()), []);
    }

    #[test]
    fn a_gray_profile_permits_only_gray() {
        let found = problems(
            "#text(fill: cmyk(0%, 0%, 0%, 80%))[Dark] #text(fill: luma(20%))[Dark]",
            colors(Colors::Gray),
        );

        assert_eq!(
            kinds(&found),
            [(
                Some(0),
                ProblemKind::Color {
                    color: "cmyk(0%, 0%, 0%, 80%)".to_owned(),
                    permitted: Colors::Gray
                }
            )]
        );
    }

    #[test]
    fn a_gradient_needs_a_permitted_space() {
        let stops = "cmyk(100%, 0%, 0%, 0%), cmyk(0%, 100%, 0%, 0%)";

        assert_eq!(
            kinds(&problems(
                &format!("#rect(fill: gradient.linear({stops}))"),
                colors(Colors::Cmyk)
            )),
            [(
                Some(0),
                ProblemKind::GradientSpace {
                    space: "oklab".to_owned(),
                    permitted: Colors::Cmyk
                }
            )]
        );
        assert_eq!(
            problems(
                &format!("#rect(fill: gradient.linear({stops}, space: cmyk))"),
                colors(Colors::Cmyk)
            ),
            []
        );
    }
}
