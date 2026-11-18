use std::io::Cursor;

use iced::window;

const WINDOW_ICON: &[u8] = include_bytes!("../icons/presswork-64.png");

pub(crate) fn window_icon() -> Option<window::Icon> {
    let (pixels, width, height) = decode(WINDOW_ICON).ok()?;
    window::icon::from_rgba(pixels, width, height).ok()
}

fn decode(data: &[u8]) -> Result<(Vec<u8>, u32, u32), png::DecodingError> {
    let mut reader = png::Decoder::new(Cursor::new(data)).read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let frame = reader.next_frame(&mut pixels)?;
    pixels.truncate(frame.buffer_size());
    Ok((pixels, frame.width, frame.height))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use resvg::tiny_skia::{Pixmap, Transform};
    use resvg::usvg::{Options, Tree};

    use super::decode;

    const SVG: &str = include_str!("../icons/presswork.svg");
    const PNG_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];
    const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
    const ICNS_IMAGES: [(&[u8; 4], u32); 11] = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"ic11", 32),
        (b"icp6", 64),
        (b"ic12", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic13", 256),
        (b"ic09", 512),
        (b"ic14", 512),
        (b"ic10", 1024),
    ];
    const MACOS_SCALE: f32 = 824.0 / 960.0;

    fn folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("icons")
    }

    fn render(size: u32, scale: f32) -> Vec<u8> {
        let tree = Tree::from_str(SVG, &Options::default()).unwrap();
        let mut pixmap = Pixmap::new(size, size).unwrap();
        let pixels = f32::from(u16::try_from(size).unwrap());
        let fit = pixels / tree.size().width() * scale;
        let offset = pixels * (1.0 - scale) / 2.0;
        let transform = Transform::from_row(fit, 0.0, 0.0, fit, offset, offset);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        pixmap.encode_png().unwrap()
    }

    fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut file = vec![0, 0, 1, 0];
        file.extend(u16::try_from(images.len()).unwrap().to_le_bytes());
        let mut start = 6 + 16 * images.len();
        for (size, image) in images {
            let side = if *size >= 256 {
                0
            } else {
                u8::try_from(*size).unwrap()
            };
            file.extend([side, side, 0, 0, 1, 0, 32, 0]);
            file.extend(u32::try_from(image.len()).unwrap().to_le_bytes());
            file.extend(u32::try_from(start).unwrap().to_le_bytes());
            start += image.len();
        }
        for (_, image) in images {
            file.extend(image);
        }
        file
    }

    fn icns(images: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (kind, image) in images {
            body.extend(*kind);
            body.extend(u32::try_from(image.len() + 8).unwrap().to_be_bytes());
            body.extend(image);
        }
        let mut file = b"icns".to_vec();
        file.extend(u32::try_from(body.len() + 8).unwrap().to_be_bytes());
        file.extend(body);
        file
    }

    fn ico_images(file: &[u8]) -> Vec<(u32, Vec<u8>)> {
        let count = usize::from(u16::from_le_bytes([file[4], file[5]]));
        (0..count)
            .map(|index| {
                let entry = &file[6 + 16 * index..6 + 16 * (index + 1)];
                let size = if entry[0] == 0 {
                    256
                } else {
                    u32::from(entry[0])
                };
                let length = number(&entry[8..12], u32::from_le_bytes);
                let start = number(&entry[12..16], u32::from_le_bytes);
                (size, file[start..start + length].to_vec())
            })
            .collect()
    }

    fn icns_images(file: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
        let mut images = Vec::new();
        let mut start = 8;
        while start < file.len() {
            let kind = file[start..start + 4].try_into().unwrap();
            let length = number(&file[start + 4..start + 8], u32::from_be_bytes);
            images.push((kind, file[start + 8..start + length].to_vec()));
            start += length;
        }
        images
    }

    fn number(bytes: &[u8], read: fn([u8; 4]) -> u32) -> usize {
        usize::try_from(read(bytes.try_into().unwrap())).unwrap()
    }

    fn assert_render(name: &str, actual: &[u8], expected: &[u8]) {
        let (actual, actual_width, actual_height) = decode(actual).unwrap();
        let (expected, width, height) = decode(expected).unwrap();
        assert_eq!((actual_width, actual_height), (width, height), "{name}");
        let difference = actual
            .iter()
            .zip(&expected)
            .map(|(left, right)| left.abs_diff(*right))
            .max();
        assert!(
            difference <= Some(2),
            "{name} is not a render of presswork.svg. Run the render_icons test."
        );
    }

    #[test]
    fn an_ico_file_lists_each_image_with_its_size_and_place() {
        let file = ico(&[(16, vec![1, 2, 3]), (256, vec![4, 5])]);

        let expected = [
            vec![0, 0, 1, 0, 2, 0],
            vec![16, 16, 0, 0, 1, 0, 32, 0, 3, 0, 0, 0, 38, 0, 0, 0],
            vec![0, 0, 0, 0, 1, 0, 32, 0, 2, 0, 0, 0, 41, 0, 0, 0],
            vec![1, 2, 3, 4, 5],
        ]
        .concat();
        assert_eq!(file, expected);
    }

    #[test]
    fn an_icns_file_gives_the_length_of_the_file_and_of_each_image() {
        let file = icns(&[(b"ic07", vec![1, 2, 3])]);

        let expected = [
            b"icns".as_slice(),
            &[0, 0, 0, 19],
            b"ic07",
            &[0, 0, 0, 11],
            &[1, 2, 3],
        ]
        .concat();
        assert_eq!(file, expected);
    }

    #[test]
    fn the_icon_files_are_renders_of_the_svg() {
        let folder = folder();
        for size in PNG_SIZES {
            let name = format!("presswork-{size}.png");
            let file = fs::read(folder.join(&name)).unwrap();
            assert_render(&name, &file, &render(size, 1.0));
        }

        let images = ico_images(&fs::read(folder.join("presswork.ico")).unwrap());
        let sizes: Vec<u32> = images.iter().map(|(size, _)| *size).collect();
        assert_eq!(sizes, ICO_SIZES);
        for (size, image) in &images {
            let name = format!("the {size} px image in presswork.ico");
            assert_render(&name, image, &render(*size, 1.0));
        }

        let images = icns_images(&fs::read(folder.join("presswork.icns")).unwrap());
        let kinds: Vec<[u8; 4]> = images.iter().map(|(kind, _)| *kind).collect();
        let expected: Vec<[u8; 4]> = ICNS_IMAGES.iter().map(|(kind, _)| **kind).collect();
        assert_eq!(kinds, expected);
        for ((kind, image), (_, size)) in images.iter().zip(ICNS_IMAGES) {
            let name = format!(
                "the {} image in presswork.icns",
                String::from_utf8_lossy(kind)
            );
            assert_render(&name, image, &render(size, MACOS_SCALE));
        }
    }

    #[test]
    #[ignore = "writes the icon files from presswork.svg"]
    fn render_icons() {
        let folder = folder();
        for size in PNG_SIZES {
            fs::write(
                folder.join(format!("presswork-{size}.png")),
                render(size, 1.0),
            )
            .unwrap();
        }
        let images: Vec<_> = ICO_SIZES
            .iter()
            .map(|&size| (size, render(size, 1.0)))
            .collect();
        fs::write(folder.join("presswork.ico"), ico(&images)).unwrap();
        let images: Vec<_> = ICNS_IMAGES
            .iter()
            .map(|&(kind, size)| (kind, render(size, MACOS_SCALE)))
            .collect();
        fs::write(folder.join("presswork.icns"), icns(&images)).unwrap();
    }
}
