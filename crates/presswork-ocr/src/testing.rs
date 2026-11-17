use std::fmt::Write as _;

pub enum TestPage {
    Text(String),
    Scan {
        width: f64,
        height: f64,
        columns: u32,
        rows: u32,
        pixels: Vec<u8>,
    },
}

fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

#[must_use]
pub fn lines(lines: &[&str]) -> String {
    let mut content = String::from("BT /F1 12 Tf 14 TL 72 770 Td\n");
    for line in lines {
        let _ = writeln!(content, "({}) Tj T*", escaped(line));
    }
    content.push_str("ET");
    content
}

#[must_use]
pub fn pdf(pages: &[String]) -> Vec<u8> {
    pdf_with_catalog(pages, "")
}

#[must_use]
pub fn pdf_with_catalog(pages: &[String], catalog: &str) -> Vec<u8> {
    let pages: Vec<TestPage> = pages.iter().cloned().map(TestPage::Text).collect();
    pdf_of(&pages, catalog)
}

fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

#[must_use]
pub fn pdf_of(pages: &[TestPage], catalog: &str) -> Vec<u8> {
    let mut kids = Vec::new();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    for page in pages {
        let id = 4 + objects.len();
        kids.push(format!("{id} 0 R"));
        match page {
            TestPage::Text(content) => {
                objects.push(
                    format!(
                        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
                         /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                        id + 1
                    )
                    .into_bytes(),
                );
                objects.push(stream("", content.as_bytes()));
            }
            TestPage::Scan {
                width,
                height,
                columns,
                rows,
                pixels,
            } => {
                objects.push(
                    format!(
                        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
                         /Resources << /XObject << /Scan {} 0 R >> >> /Contents {} 0 R >>",
                        id + 2,
                        id + 1
                    )
                    .into_bytes(),
                );
                let content = format!("q {width} 0 0 {height} 0 0 cm /Scan Do Q");
                objects.push(stream("", content.as_bytes()));
                objects.push(stream(
                    &format!(
                        "/Type /XObject /Subtype /Image /Width {columns} /Height {rows} \
                         /ColorSpace /DeviceGray /BitsPerComponent 8"
                    ),
                    pixels,
                ));
            }
        }
    }
    let mut all = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>").into_bytes(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    ];
    all.extend(objects);
    let mut file = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in all.iter().enumerate() {
        offsets.push(file.len());
        file.extend(format!("{} 0 obj\n", index + 1).as_bytes());
        file.extend(object);
        file.extend(b"\nendobj\n");
    }
    let start = file.len();
    file.extend(format!("xref\n0 {}\n0000000000 65535 f \n", all.len() + 1).as_bytes());
    for offset in offsets {
        file.extend(format!("{offset:010} 00000 n \n").as_bytes());
    }
    file.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
            all.len() + 1
        )
        .as_bytes(),
    );
    file
}
