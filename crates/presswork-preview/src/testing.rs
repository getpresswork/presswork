use std::path::Path;

pub(crate) struct Picture {
    width: usize,
    bytes: Vec<u8>,
}

impl Picture {
    pub(crate) fn read(folder: &Path) -> Self {
        let file = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.extension().is_some_and(|extension| extension == "png"))
            .unwrap();
        let decoder =
            png::Decoder::new(std::io::BufReader::new(std::fs::File::open(file).unwrap()));
        let mut reader = decoder.read_info().unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        Self {
            width: info.width as usize,
            bytes,
        }
    }

    pub(crate) fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let at = (y * self.width + x) * 4;
        [
            self.bytes[at],
            self.bytes[at + 1],
            self.bytes[at + 2],
            self.bytes[at + 3],
        ]
    }
}

pub(crate) fn pixel(folder: &Path, x: usize, y: usize) -> [u8; 4] {
    Picture::read(folder).pixel(x, y)
}
