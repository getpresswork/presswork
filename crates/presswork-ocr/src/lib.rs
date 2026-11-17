mod engine;
mod labels;
mod languages;
mod ocrmypdf;
#[cfg(feature = "ocrs")]
mod ocrs_engine;
#[cfg(any(test, feature = "ocrs"))]
mod render;
mod text_layer;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use engine::{OcrEngine, OcrError, Recognized};
pub use languages::{languages_for, tesseract_language};
pub use ocrmypdf::OcrMyPdf;
#[cfg(feature = "ocrs")]
pub use ocrs_engine::{DETECTION_MODEL, ModelFile, Ocrs, RECOGNITION_MODEL};
pub use text_layer::{PdfPages, ReadError, page_count, read_pages, text_layer};
