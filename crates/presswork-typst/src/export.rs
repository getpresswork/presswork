use typst::foundations::Datetime;
use typst::utils::Scalar;
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};
use typst_render::RenderOptions;

use crate::ide::Analysis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Pdf,
    Png,
    Svg,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    pub format: Format,
    pub standards: Vec<String>,
    pub pixel_per_pt: f32,
    pub date: Option<Datetime>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    #[error("the PDF standard {0} is not known")]
    UnknownStandard(String),
    #[error("{0}")]
    Standards(String),
    #[error("{}", .0.join("; "))]
    Failed(Vec<String>),
    #[error("a page image did not encode: {0}")]
    Encode(String),
    #[error("the document has errors, so it has no pages")]
    NoPages,
}

impl Analysis {
    pub fn export(&self, settings: &ExportSettings) -> Result<Vec<Vec<u8>>, ExportError> {
        export(self.document().ok_or(ExportError::NoPages)?, settings)
    }
}

pub fn export(
    document: &PagedDocument,
    settings: &ExportSettings,
) -> Result<Vec<Vec<u8>>, ExportError> {
    match settings.format {
        Format::Pdf => {
            let list = settings
                .standards
                .iter()
                .map(|name| {
                    standard(name).ok_or_else(|| ExportError::UnknownStandard(name.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let standards = PdfStandards::new(&list)
                .map_err(|error| ExportError::Standards(error.message().to_string()))?;
            let options = PdfOptions {
                standards,
                timestamp: settings.date.map(Timestamp::new_utc),
                ..PdfOptions::default()
            };
            typst_pdf::pdf(document, &options)
                .map(|bytes| vec![bytes])
                .map_err(|problems| {
                    ExportError::Failed(
                        problems
                            .iter()
                            .map(|problem| problem.message.to_string())
                            .collect(),
                    )
                })
        }
        Format::Png => {
            let options = RenderOptions {
                pixel_per_pt: Scalar::new(f64::from(settings.pixel_per_pt)),
                render_bleed: false,
            };
            document
                .pages()
                .iter()
                .map(|page| {
                    typst_render::render(page, &options)
                        .encode_png()
                        .map_err(|error| ExportError::Encode(error.to_string()))
                })
                .collect()
        }
        Format::Svg => Ok(document
            .pages()
            .iter()
            .map(|page| typst_svg::svg(page, &typst_svg::SvgOptions::default()).into_bytes())
            .collect()),
    }
}

fn standard(name: &str) -> Option<PdfStandard> {
    Some(match name {
        "1.4" => PdfStandard::V_1_4,
        "1.5" => PdfStandard::V_1_5,
        "1.6" => PdfStandard::V_1_6,
        "1.7" => PdfStandard::V_1_7,
        "2.0" => PdfStandard::V_2_0,
        "a-1b" => PdfStandard::A_1b,
        "a-1a" => PdfStandard::A_1a,
        "a-2b" => PdfStandard::A_2b,
        "a-2u" => PdfStandard::A_2u,
        "a-2a" => PdfStandard::A_2a,
        "a-3b" => PdfStandard::A_3b,
        "a-3u" => PdfStandard::A_3u,
        "a-3a" => PdfStandard::A_3a,
        "a-4" => PdfStandard::A_4,
        "a-4f" => PdfStandard::A_4f,
        "a-4e" => PdfStandard::A_4e,
        "ua-1" => PdfStandard::Ua_1,
        _ => return None,
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
        let today = Datetime::from_ymd(2026, 11, 20).unwrap();
        let mut world = World::new(Arc::new(Fonts::embedded()), Some(today));
        world.set_main_text(text);
        compile(&world).document.unwrap()
    }

    fn settings(format: Format, standards: &[&str]) -> ExportSettings {
        ExportSettings {
            format,
            standards: standards.iter().map(|name| (*name).to_owned()).collect(),
            pixel_per_pt: 1.0,
            date: Datetime::from_ymd_hms(2026, 11, 20, 12, 0, 0),
        }
    }

    const TWO_PAGES: &str = "One.\n#pagebreak()\nTwo.";

    #[test]
    fn a_pdf_export_gives_one_pdf_file() {
        let files = export(&document(TWO_PAGES), &settings(Format::Pdf, &[])).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].starts_with(b"%PDF-"));
    }

    #[test]
    fn a_png_export_gives_one_image_for_each_page() {
        let files = export(&document(TWO_PAGES), &settings(Format::Png, &[])).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|file| file.starts_with(b"\x89PNG")));
    }

    #[test]
    fn an_svg_export_gives_one_drawing_for_each_page() {
        let files = export(&document(TWO_PAGES), &settings(Format::Svg, &[])).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|file| file.starts_with(b"<svg")));
    }

    #[test]
    fn a_pdf_a_export_tells_its_standard() {
        let files = export(
            &document("#set document(title: [Book])\nText."),
            &settings(Format::Pdf, &["a-2b"]),
        )
        .unwrap();

        let text = String::from_utf8_lossy(&files[0]);
        assert!(text.contains("pdfaid"), "no PDF/A identification");
    }

    #[test]
    fn standards_that_do_not_agree_give_an_error() {
        let result = export(
            &document("Text."),
            &settings(Format::Pdf, &["a-2b", "a-3b"]),
        );

        assert!(
            matches!(result, Err(ExportError::Standards(_))),
            "{result:?}"
        );
    }

    #[test]
    fn an_unknown_standard_gives_an_error() {
        let result = export(&document("Text."), &settings(Format::Pdf, &["x-9"]));

        assert_eq!(result, Err(ExportError::UnknownStandard("x-9".to_owned())));
    }

    fn analysis(text: &str) -> crate::Analysis {
        let today = Datetime::from_ymd(2026, 11, 20);
        let world = std::sync::Mutex::new(World::new(Arc::new(Fonts::embedded()), today));
        crate::job::analyze(&world, text).analysis.unwrap()
    }

    #[test]
    fn a_compilation_exports_its_pages() {
        let files = analysis(TWO_PAGES)
            .export(&settings(Format::Svg, &[]))
            .unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn a_compilation_with_errors_has_no_pages_to_export() {
        let result = analysis("#let").export(&settings(Format::Pdf, &[]));

        assert_eq!(result, Err(ExportError::NoPages));
    }

    #[test]
    fn a_document_that_breaks_its_standard_gives_the_problems() {
        let result = export(&document("Text."), &settings(Format::Pdf, &["ua-1"]));

        assert!(
            matches!(&result, Err(ExportError::Failed(problems)) if !problems.is_empty()),
            "{result:?}"
        );
    }
}
