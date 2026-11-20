use std::collections::HashSet;

use presswork_project::{Colors, ExportProfile, PageSize};
use serde::Serialize;
use typst::foundations::Repr as _;
use typst::layout::{Abs, Frame, FrameItem, Size};
use typst::syntax::Span;
use typst::text::TextItem;
use typst::visualize::{Color, ColorSpace, Image, ImageKind, Paint, ProcessColorSpace};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub page: Option<usize>,
    pub kind: ProblemKind,
    pub source: Option<SourceRange>,
}

impl Output {
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
        };
        if let Some(document) = self.document() {
            for (page, content) in document.pages().iter().enumerate() {
                check.page = page;
                check.page_size(&content.frame);
                check.frame(&content.frame, (1.0, 1.0));
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
}

impl Check<'_> {
    fn push(&mut self, kind: ProblemKind, source: Option<SourceRange>) {
        self.problems.push(Problem {
            page: Some(self.page),
            kind,
            source,
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
            );
        }
    }

    fn frame(&mut self, frame: &Frame, scale: (f64, f64)) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => {
                    let transform = group.transform;
                    let scale_x = transform.sx.get().hypot(transform.ky.get()) * scale.0;
                    let scale_y = transform.kx.get().hypot(transform.sy.get()) * scale.1;
                    self.frame(&group.frame, (scale_x, scale_y));
                }
                FrameItem::Text(text) => self.text(text, scale),
                FrameItem::Shape(shape, span) => {
                    if let Some(fill) = &shape.fill {
                        self.paint(fill, *span, scale);
                    }
                    if let Some(stroke) = &shape.stroke {
                        self.paint(&stroke.paint, *span, scale);
                    }
                }
                FrameItem::Image(image, size, span) => self.image(image, *size, *span, scale),
                FrameItem::Link(..) | FrameItem::Tag(_) => {}
            }
        }
    }

    fn text(&mut self, text: &TextItem, scale: (f64, f64)) {
        let span = text
            .glyphs
            .first()
            .map_or_else(Span::detached, |glyph| glyph.span.0);
        self.paint(&text.fill, span, scale);
        if let Some(stroke) = &text.stroke {
            self.paint(&stroke.paint, span, scale);
        }
        for glyph in text.glyphs.iter().filter(|glyph| glyph.id == 0) {
            let range = usize::from(glyph.range.start)..usize::from(glyph.range.end);
            let characters = text.text.get(range).unwrap_or_default();
            for character in characters
                .chars()
                .filter(|character| !character.is_whitespace())
            {
                if self.missing_source.is_none() {
                    self.missing_source = self.analysis.source_range(glyph.span.0);
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
        self.push(ProblemKind::MissingGlyphs { characters }, source);
    }

    fn paint(&mut self, paint: &Paint, span: Span, scale: (f64, f64)) {
        let Some(permitted) = self.rules.colors else {
            return;
        };
        match paint {
            Paint::Solid(color) => self.color(color, permitted, span),
            Paint::Gradient(gradient) => {
                let space = gradient.space();
                if !permits(permitted, &space) {
                    let space = space_name(&space).to_owned();
                    if self.spaces.insert(space.clone()) {
                        let source = self.analysis.source_range(span);
                        self.push(ProblemKind::GradientSpace { space, permitted }, source);
                    }
                }
            }
            Paint::Tiling(tiling) => self.frame(tiling.frame(), scale),
        }
    }

    fn color(&mut self, color: &Color, permitted: Colors, span: Span) {
        if permits(permitted, &color.space()) {
            return;
        }
        let color = color.repr().to_string();
        if self.colors.insert(color.clone()) {
            let source = self.analysis.source_range(span);
            self.push(ProblemKind::Color { color, permitted }, source);
        }
    }

    fn image(&mut self, image: &Image, size: Size, span: Span, scale: (f64, f64)) {
        let source = self.analysis.source_range(span);
        if image.alt().is_none() {
            self.push(ProblemKind::MissingAltText, source.clone());
        }
        if let (Some(minimum), ImageKind::Raster(raster)) = (self.rules.minimum_ppi, image.kind()) {
            let across = f64::from(raster.width()) / (size.x.to_pt() * scale.0 / POINTS_PER_INCH);
            let down = f64::from(raster.height()) / (size.y.to_pt() * scale.1 / POINTS_PER_INCH);
            let ppi = whole(across.min(down));
            if ppi < minimum {
                self.push(ProblemKind::LowResolution { ppi, minimum }, source);
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

    use presswork_project::LengthUnit;
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

    fn problems(text: &str, rules: Rules) -> Vec<Problem> {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("main.typ"), text).unwrap();
        fs::write(folder.path().join("dot.png"), png(100, 100)).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        run(&world, text, 1.0, false).preflight(&rules)
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
