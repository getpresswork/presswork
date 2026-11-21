use typst::layout::{Abs, Em, Point, Rect, Transform};
use typst::text::TextItem;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Area {
    pub(crate) fn of(rect: Rect, transform: Transform) -> Self {
        let corners = [
            rect.min,
            Point::new(rect.max.x, rect.min.y),
            rect.max,
            Point::new(rect.min.x, rect.max.y),
        ]
        .map(|corner| corner.transform(transform));
        let xs = corners.map(|corner| corner.x.to_pt());
        let ys = corners.map(|corner| corner.y.to_pt());
        let left = xs.into_iter().fold(f64::INFINITY, f64::min);
        let top = ys.into_iter().fold(f64::INFINITY, f64::min);
        Self {
            x: left,
            y: top,
            width: xs.into_iter().fold(f64::NEG_INFINITY, f64::max) - left,
            height: ys.into_iter().fold(f64::NEG_INFINITY, f64::max) - top,
        }
    }

    pub(crate) fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            width: (self.x + self.width).max(other.x + other.width) - x,
            height: (self.y + self.height).max(other.y + other.height) - y,
        }
    }
}

pub(crate) fn text_rect(text: &TextItem) -> Rect {
    line_rect(text, Abs::zero(), text.width())
}

pub(crate) fn glyph_rect(text: &TextItem, index: usize) -> Rect {
    let start = text.glyphs[..index]
        .iter()
        .map(|glyph| glyph.x_advance)
        .sum::<Em>()
        .at(text.size);
    let width = text
        .glyphs
        .get(index)
        .map_or(Abs::zero(), |glyph| glyph.x_advance.at(text.size));
    line_rect(text, start, start + width)
}

fn line_rect(text: &TextItem, start: Abs, end: Abs) -> Rect {
    let metrics = text.font.metrics();
    Rect::new(
        Point::new(start, -metrics.ascender.at(text.size)),
        Point::new(end, -metrics.descender.at(text.size)),
    )
}
