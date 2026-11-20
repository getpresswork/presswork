use typst::layout::{Frame, FrameItem};
use typst::visualize::ImageKind;

use crate::ide::Analysis;
use crate::outline::SourceRange;

const POINTS_PER_INCH: f64 = 72.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    LowResolution { ppi: u32, minimum: u32 },
    MissingAltText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub page: usize,
    pub kind: ProblemKind,
    pub source: Option<SourceRange>,
}

impl Analysis {
    #[must_use]
    pub fn preflight(&self, minimum_ppi: Option<u32>) -> Vec<Problem> {
        let mut problems = Vec::new();
        if let Some(document) = self.document() {
            for (page, content) in document.pages().iter().enumerate() {
                self.check(&content.frame, (1.0, 1.0), page, minimum_ppi, &mut problems);
            }
        }
        problems
    }

    fn check(
        &self,
        frame: &Frame,
        scale: (f64, f64),
        page: usize,
        minimum_ppi: Option<u32>,
        problems: &mut Vec<Problem>,
    ) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => {
                    let transform = group.transform;
                    let scale_x = transform.sx.get().hypot(transform.ky.get()) * scale.0;
                    let scale_y = transform.kx.get().hypot(transform.sy.get()) * scale.1;
                    self.check(
                        &group.frame,
                        (scale_x, scale_y),
                        page,
                        minimum_ppi,
                        problems,
                    );
                }
                FrameItem::Image(image, size, span) => {
                    let source = self.source_range(*span);
                    if image.alt().is_none() {
                        problems.push(Problem {
                            page,
                            kind: ProblemKind::MissingAltText,
                            source: source.clone(),
                        });
                    }
                    if let (Some(minimum), ImageKind::Raster(raster)) = (minimum_ppi, image.kind())
                    {
                        let across = f64::from(raster.width())
                            / (size.x.to_pt() * scale.0 / POINTS_PER_INCH);
                        let down = f64::from(raster.height())
                            / (size.y.to_pt() * scale.1 / POINTS_PER_INCH);
                        let ppi = whole(across.min(down));
                        if ppi < minimum {
                            problems.push(Problem {
                                page,
                                kind: ProblemKind::LowResolution { ppi, minimum },
                                source,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
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

    fn problems(text: &str, minimum: Option<u32>) -> Vec<Problem> {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("main.typ"), text).unwrap();
        fs::write(folder.path().join("dot.png"), png(100, 100)).unwrap();
        let world = Mutex::new(World::open(
            Arc::new(Fonts::embedded()),
            Datetime::from_ymd(2026, 11, 20),
            folder.path().to_path_buf(),
            Path::new("main.typ"),
        ));
        run(&world, text, 1.0, false)
            .analysis
            .unwrap()
            .preflight(minimum)
    }

    fn kinds(problems: &[Problem]) -> Vec<(usize, ProblemKind)> {
        problems
            .iter()
            .map(|problem| (problem.page, problem.kind))
            .collect()
    }

    #[test]
    fn a_small_image_at_a_large_size_has_a_low_resolution() {
        let found = problems("#image(\"dot.png\", width: 1in, alt: \"A dot\")", Some(300));

        assert_eq!(
            kinds(&found),
            [(
                0,
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
            Some(300),
        );

        assert_eq!(found, []);
    }

    #[test]
    fn a_scaled_image_counts_its_scale() {
        let found = problems(
            "#scale(50%)[#image(\"dot.png\", width: 1in, alt: \"A dot\")]",
            Some(300),
        );

        assert_eq!(
            kinds(&found),
            [(
                0,
                ProblemKind::LowResolution {
                    ppi: 200,
                    minimum: 300
                }
            )]
        );
    }

    #[test]
    fn an_image_without_alternative_text_is_a_problem() {
        let found = problems("Text.\n#pagebreak()\n#image(\"dot.png\", width: 1in)", None);

        assert_eq!(kinds(&found), [(1, ProblemKind::MissingAltText)]);
    }
}
