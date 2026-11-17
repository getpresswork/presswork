use std::sync::Arc;

use hayro_interpret::font::Glyph;
use hayro_interpret::hayro_cmap::BfString;
use hayro_interpret::hayro_syntax::{LoadPdfError, Pdf};
use hayro_interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache,
    InterpreterSettings, Paint, PathDrawMode, SoftMask, TransformExt, interpret_page,
};
use kurbo::{Affine, BezPath, Point, Rect};

const GLYPH_UNITS: f64 = 1000.0;
const NEW_LINE: f64 = 0.5;
const WORD_GAP: f64 = 0.2;
const SAME_PLACE: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    #[error("the file is not a PDF file that Presswork can read")]
    Invalid,
    #[error("the PDF file is encrypted")]
    Encrypted,
}

fn open(pdf: impl AsRef<[u8]> + Send + Sync + 'static) -> Result<Pdf, ReadError> {
    Pdf::new(Arc::new(pdf)).map_err(|error| match error {
        LoadPdfError::Decryption(_) => ReadError::Encrypted,
        LoadPdfError::Invalid => ReadError::Invalid,
    })
}

pub fn page_count(pdf: impl AsRef<[u8]> + Send + Sync + 'static) -> Result<u32, ReadError> {
    let pdf = open(pdf)?;
    Ok(u32::try_from(pdf.pages().len()).unwrap_or(u32::MAX))
}

pub fn text_layer(pdf: impl AsRef<[u8]> + Send + Sync + 'static) -> Result<Vec<String>, ReadError> {
    let pdf = open(pdf)?;
    let cache = InterpreterCache::new();
    let settings = InterpreterSettings {
        render_annotations: false,
        ..InterpreterSettings::default()
    };
    let pages = pdf
        .pages()
        .iter()
        .map(|page| {
            let (width, height) = page.render_dimensions();
            let mut context = Context::new(
                page.initial_transform(true).to_kurbo(),
                Rect::new(0.0, 0.0, f64::from(width), f64::from(height)),
                &cache,
                page.xref(),
                settings.clone(),
            );
            let mut device = TextDevice::default();
            interpret_page(page, &mut context, &mut device);
            device.text.trim_end().to_owned()
        })
        .collect();
    Ok(pages)
}

#[derive(Clone, Copy)]
struct Placed {
    origin: Point,
    end: Point,
    size: f64,
}

#[derive(Default)]
struct TextDevice {
    text: String,
    last: Option<Placed>,
}

impl TextDevice {
    fn add(&mut self, text: &str, placed: Placed) {
        if let Some(last) = self.last {
            let size = placed.size.max(last.size);
            if placed.origin.distance(last.origin) < SAME_PLACE * size && self.text.ends_with(text)
            {
                return;
            }
            if (placed.origin.y - last.origin.y).abs() > NEW_LINE * size {
                self.new_line();
            } else if placed.origin.x - last.end.x > WORD_GAP * size {
                self.space();
            }
        }
        self.text.push_str(text);
        self.last = Some(placed);
    }

    fn new_line(&mut self) {
        let kept = self.text.trim_end_matches(' ').len();
        self.text.truncate(kept);
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
    }

    fn space(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with(char::is_whitespace) {
            self.text.push(' ');
        }
    }
}

impl<'a> Device<'a> for TextDevice {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}

    fn set_blend_mode(&mut self, _: BlendMode) {}

    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {}

    fn push_clip_path(&mut self, _: &ClipPath) {}

    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}

    fn draw_glyph(
        &mut self,
        glyph: &Glyph<'a>,
        transform: Affine,
        glyph_transform: Affine,
        _: &Paint<'a>,
        _: &GlyphDrawMode,
    ) {
        let text = match glyph.as_unicode() {
            Some(BfString::Char(character)) => character.to_string(),
            Some(BfString::String(text)) => text,
            None => return,
        };
        let full = transform * glyph_transform;
        let [_, _, c, d, _, _] = full.as_coeffs();
        let advance = match glyph {
            Glyph::Outline(outline) => outline.advance_width().map_or(0.0, f64::from),
            Glyph::Type3(_) => 0.0,
        };
        self.add(
            &text,
            Placed {
                origin: full * Point::ORIGIN,
                end: full * Point::new(advance, 0.0),
                size: GLYPH_UNITS * c.hypot(d),
            },
        );
    }

    fn draw_image(&mut self, _: Image<'a, '_>, _: Affine) {}

    fn pop_clip_path(&mut self) {}

    fn pop_transparency_group(&mut self) {}
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn escaped(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
    }

    fn lines(lines: &[&str]) -> String {
        let mut content = String::from("BT /F1 12 Tf 14 TL 72 770 Td\n");
        for line in lines {
            writeln!(content, "({}) Tj T*", escaped(line)).unwrap();
        }
        content.push_str("ET");
        content
    }

    fn pdf(pages: &[String]) -> Vec<u8> {
        let kids: Vec<String> = (0..pages.len())
            .map(|index| format!("{} 0 R", 4 + 2 * index))
            .collect();
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids.join(" "),
                pages.len()
            ),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
                .to_owned(),
        ];
        for (index, content) in pages.iter().enumerate() {
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
                 /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                5 + 2 * index
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ));
        }
        let mut file = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(file.len());
            file.extend(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
        }
        let start = file.len();
        file.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            file.extend(format!("{offset:010} 00000 n \n").as_bytes());
        }
        file.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        file
    }

    #[test]
    fn each_page_gives_its_text() {
        let file = pdf(&[
            lines(&["Good books start with good paper."]),
            lines(&["The ink must dry."]),
        ]);

        assert_eq!(
            text_layer(file).unwrap(),
            ["Good books start with good paper.", "The ink must dry."]
        );
    }

    #[test]
    fn the_lines_of_a_page_stay_apart() {
        let file = pdf(&[lines(&["Paper and ink", "Weights"])]);

        assert_eq!(text_layer(file).unwrap(), ["Paper and ink\nWeights"]);
    }

    #[test]
    fn a_gap_between_two_words_gives_a_space() {
        let file = pdf(&["BT /F1 12 Tf 72 770 Td [(Good) -300 (paper)] TJ ET".to_owned()]);

        assert_eq!(text_layer(file).unwrap(), ["Good paper"]);
    }

    #[test]
    fn a_page_without_text_gives_an_empty_text() {
        let file = pdf(&[String::new(), lines(&["Text."])]);

        assert_eq!(text_layer(file).unwrap(), ["", "Text."]);
    }

    #[test]
    fn filled_and_stroked_text_counts_one_time() {
        let file = pdf(&["BT /F1 12 Tf 2 Tr 72 770 Td (Bold) Tj ET".to_owned()]);

        assert_eq!(text_layer(file).unwrap(), ["Bold"]);
    }

    #[test]
    fn invisible_text_counts() {
        let file = pdf(&["BT /F1 12 Tf 3 Tr 72 770 Td (Scanned page) Tj ET".to_owned()]);

        assert_eq!(text_layer(file).unwrap(), ["Scanned page"]);
    }

    #[test]
    fn a_pdf_file_from_typst_gives_its_words() {
        let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");

        let pages = text_layer(include_bytes!("../fixtures/typst.pdf").as_slice()).unwrap();

        assert_eq!(
            pages.iter().map(|page| words(page)).collect::<Vec<_>>(),
            [
                "Paper and ink Good books start with good paper. \
                 The fine grain of the paper gives the page its Größe.",
                "The ink must dry before you fold the sheets.",
            ]
        );
        assert!(pages[0].starts_with("Paper and ink\n"), "{:?}", pages[0]);
    }

    #[test]
    fn the_page_count_includes_pages_without_text() {
        let file = pdf(&[lines(&["One"]), String::new(), lines(&["Three"])]);

        assert_eq!(page_count(file).unwrap(), 3);
        assert_eq!(
            page_count(include_bytes!("../fixtures/typst.pdf").as_slice()).unwrap(),
            2
        );
    }

    #[test]
    fn a_file_that_is_not_a_pdf_file_has_no_page_count() {
        assert!(matches!(
            page_count(b"Good paper".to_vec()),
            Err(ReadError::Invalid)
        ));
    }

    #[test]
    fn a_file_that_is_not_a_pdf_file_is_an_error() {
        assert!(matches!(
            text_layer(b"Good paper".to_vec()),
            Err(ReadError::Invalid)
        ));
    }
}
