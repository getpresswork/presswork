use hayro_interpret::hayro_syntax::Pdf;
use hayro_interpret::hayro_syntax::object::{Array, Dict, Name, String as PdfString};
use presswork_api::{LabelStyle, PageRange};

const MAX_DEPTH: u32 = 32;

pub(crate) fn label_ranges(pdf: &Pdf) -> Vec<PageRange> {
    let xref = pdf.xref();
    let Some(catalog) = xref.get::<Dict<'_>>(xref.root_id()) else {
        return Vec::new();
    };
    let Some(tree) = catalog.get::<Dict<'_>>(b"PageLabels") else {
        return Vec::new();
    };
    let mut ranges = Vec::new();
    collect(&tree, &mut ranges, 0);
    ranges.sort_by_key(|range| range.first_page);
    ranges.dedup_by_key(|range| range.first_page);
    ranges
}

fn collect(node: &Dict<'_>, ranges: &mut Vec<PageRange>, depth: u32) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(numbers) = node.get::<Array<'_>>(b"Nums") {
        let mut items = numbers.flex_iter();
        while let (Some(index), Some(label)) = (items.next::<u32>(), items.next::<Dict<'_>>()) {
            ranges.push(range(index, &label));
        }
    }
    if let Some(kids) = node.get::<Array<'_>>(b"Kids") {
        for kid in kids.iter::<Dict<'_>>() {
            collect(&kid, ranges, depth + 1);
        }
    }
}

fn range(index: u32, label: &Dict<'_>) -> PageRange {
    let style = match label.get::<Name<'_>>(b"S").as_deref() {
        Some(b"D") => LabelStyle::Arabic,
        Some(b"R") => LabelStyle::UpperRoman,
        Some(b"r") => LabelStyle::LowerRoman,
        Some(b"A") => LabelStyle::UpperLetters,
        Some(b"a") => LabelStyle::LowerLetters,
        _ => LabelStyle::None,
    };
    PageRange {
        first_page: index.saturating_add(1),
        style,
        prefix: label
            .get::<PdfString<'_>>(b"P")
            .as_deref()
            .map_or_else(String::new, text_string),
        start: label.get::<u32>(b"St").unwrap_or(1).max(1),
    }
}

fn text_string(bytes: &[u8]) -> String {
    match bytes.strip_prefix(&[0xFE, 0xFF]) {
        Some(utf16) => {
            let (pairs, _) = utf16.as_chunks::<2>();
            let units: Vec<u16> = pairs.iter().map(|pair| u16::from_be_bytes(*pair)).collect();
            String::from_utf16_lossy(&units)
        }
        None => bytes.iter().map(|&byte| char::from(byte)).collect(),
    }
}
