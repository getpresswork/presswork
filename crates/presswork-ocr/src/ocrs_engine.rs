use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::Path;

use hayro::RenderCache;
use ocrs::{ImageSource, OcrEngineParams};
use rten::Model;
use sha2::{Digest, Sha256};

use crate::engine::{OcrEngine, OcrError, Recognized};
use crate::render::page_image;
use crate::text_layer::{open, texts};

const DPI: f32 = 200.0;

pub struct ModelFile {
    pub name: &'static str,
    pub url: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

pub const DETECTION_MODEL: ModelFile = ModelFile {
    name: "text-detection.onnx",
    url: "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx",
    size: 2_499_479,
    sha256: "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe",
};

pub const RECOGNITION_MODEL: ModelFile = ModelFile {
    name: "text-recognition.onnx",
    url: "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx",
    size: 9_713_177,
    sha256: "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5",
};

impl ModelFile {
    #[must_use]
    pub fn matches(&self, data: &[u8]) -> bool {
        let digest = Sha256::digest(data)
            .iter()
            .fold(String::new(), |mut text, byte| {
                let _ = write!(text, "{byte:02x}");
                text
            });
        u64::try_from(data.len()).is_ok_and(|size| size == self.size) && digest == self.sha256
    }

    fn load(&self, folder: &Path) -> Result<Model, OcrError> {
        let data = std::fs::read(folder.join(self.name)).map_err(|error| match error.kind() {
            ErrorKind::NotFound => {
                OcrError::Engine(format!("the model file {} is missing", self.name))
            }
            _ => OcrError::Io(error),
        })?;
        if !self.matches(&data) {
            return Err(OcrError::Engine(format!(
                "the model file {} is not the expected file",
                self.name
            )));
        }
        Model::load(data).map_err(|error| OcrError::Engine(error.to_string()))
    }
}

fn engine_error(error: &dyn std::error::Error) -> OcrError {
    OcrError::Engine(error.to_string())
}

pub struct Ocrs {
    engine: ocrs::OcrEngine,
}

impl Ocrs {
    pub fn from_folder(folder: &Path) -> Result<Self, OcrError> {
        let engine = ocrs::OcrEngine::new(OcrEngineParams {
            detection_model: Some(DETECTION_MODEL.load(folder)?),
            recognition_model: Some(RECOGNITION_MODEL.load(folder)?),
            ..OcrEngineParams::default()
        })
        .map_err(|error| engine_error(&error))?;
        Ok(Self { engine })
    }
}

impl OcrEngine for Ocrs {
    fn languages(&self) -> &[String] {
        &[]
    }

    fn recognize(&self, pdf: &[u8], _languages: &[String]) -> Result<Recognized, OcrError> {
        let document = open(pdf.to_vec())?;
        let mut texts = texts(&document);
        let cache = RenderCache::new();
        for (page, text) in document.pages().iter().zip(texts.iter_mut()) {
            if !text.trim().is_empty() {
                continue;
            }
            let image = page_image(page, &cache, DPI);
            let source = ImageSource::from_bytes(&image.pixels, (image.columns, image.rows))
                .map_err(|error| engine_error(&error))?;
            let input = self
                .engine
                .prepare_input(source)
                .map_err(|error| engine_error(&error))?;
            let found = self
                .engine
                .get_text(&input)
                .map_err(|error| engine_error(&error))?;
            found.trim_end().clone_into(text);
        }
        Ok(Recognized { pdf: None, texts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestPage, lines, pdf_of};

    #[test]
    fn a_model_file_matches_only_its_size_and_digest() {
        let model = ModelFile {
            name: "test.onnx",
            url: "",
            size: 4,
            sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        };

        assert!(model.matches(b"test"));
        assert!(!model.matches(b"tost"));
        assert!(!model.matches(b"tests"));
        assert!(!DETECTION_MODEL.matches(b"test"));
    }

    #[test]
    fn a_folder_without_the_models_cannot_make_the_engine() {
        let folder = tempfile::tempdir().unwrap();

        let missing = Ocrs::from_folder(folder.path()).err().unwrap();
        assert!(
            matches!(&missing, OcrError::Engine(reason) if reason.contains("text-detection.onnx"))
        );

        std::fs::write(folder.path().join(DETECTION_MODEL.name), b"other data").unwrap();
        let changed = Ocrs::from_folder(folder.path()).err().unwrap();
        assert!(
            matches!(&changed, OcrError::Engine(reason) if reason.contains("not the expected")),
            "{changed:?}"
        );
    }

    fn words(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn ocrs_gives_the_text_of_the_scanned_pages() {
        let Some(folder) = std::env::var_os("PRESSWORK_TEST_OCRS_MODELS") else {
            eprintln!("PRESSWORK_TEST_OCRS_MODELS is not set. This test does not run.");
            return;
        };
        let engine = Ocrs::from_folder(Path::new(&folder)).unwrap();
        let scan = pdf_of(
            &[
                crate::render::scan(include_bytes!("../fixtures/typst.pdf"), 0),
                TestPage::Text(lines(&["The ink must dry."])),
            ],
            "",
        );

        let recognized = engine.recognize(&scan, &[]).unwrap();

        assert_eq!(recognized.pdf, None);
        assert_eq!(recognized.texts.len(), 2);
        assert!(
            words(&recognized.texts[0]).contains("Good books start with good paper."),
            "{:?}",
            recognized.texts[0]
        );
        assert_eq!(recognized.texts[1], "The ink must dry.");
    }
}
