use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::page::Page;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings};

pub(crate) struct GrayImage {
    pub(crate) columns: u32,
    pub(crate) rows: u32,
    pub(crate) pixels: Vec<u8>,
}

pub(crate) fn page_image<'a>(page: &'a Page<'a>, cache: &RenderCache<'a>, dpi: f32) -> GrayImage {
    let scale = dpi / 72.0;
    let pixmap = hayro::render(
        page,
        cache,
        &InterpreterSettings::default(),
        &RenderSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
            ..RenderSettings::default()
        },
    );
    GrayImage {
        columns: u32::from(pixmap.width()),
        rows: u32::from(pixmap.height()),
        pixels: pixmap
            .data()
            .iter()
            .map(|pixel| luma(pixel.r, pixel.g, pixel.b))
            .collect(),
    }
}

fn luma(red: u8, green: u8, blue: u8) -> u8 {
    let value = (299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue)) / 1000;
    u8::try_from(value).unwrap_or(u8::MAX)
}

#[cfg(test)]
pub(crate) fn scan(pdf: &'static [u8], index: usize) -> crate::testing::TestPage {
    let document = crate::text_layer::open(pdf).unwrap();
    let page = &document.pages()[index];
    let (width, height) = page.render_dimensions();
    let image = page_image(page, &RenderCache::new(), 200.0);
    crate::testing::TestPage::Scan {
        width: f64::from(width),
        height: f64::from(height),
        columns: image.columns,
        rows: image.rows,
        pixels: image.pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luma_weights_green_most() {
        assert_eq!(luma(255, 255, 255), 255);
        assert_eq!(luma(0, 0, 0), 0);
        assert_eq!(luma(0, 255, 0), 149);
        assert_eq!(luma(255, 0, 0), 76);
    }

    #[test]
    fn a_scan_of_a_page_is_an_image_of_its_size() {
        let crate::testing::TestPage::Scan {
            width,
            height,
            columns,
            rows,
            pixels,
        } = scan(include_bytes!("../fixtures/typst.pdf"), 0)
        else {
            panic!("a scan is an image");
        };

        assert_eq!((width, height), (320.0, 200.0));
        assert_eq!((columns, rows), (888, 555));
        assert_eq!(pixels.len(), 888 * 555);
        assert!(pixels.iter().any(|&pixel| pixel < 100));
        let white: usize = pixels
            .iter()
            .map(|&pixel| usize::from(pixel == u8::MAX))
            .sum();
        assert!(white > pixels.len() / 2);
    }
}
