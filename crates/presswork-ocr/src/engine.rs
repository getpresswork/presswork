use std::time::Duration;

use crate::ReadError;

pub trait OcrEngine: Send + Sync {
    fn languages(&self) -> &[String];

    fn recognize(&self, pdf: &[u8], languages: &[String]) -> Result<Recognized, OcrError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recognized {
    pub pdf: Option<Vec<u8>>,
    pub texts: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("the OCR engine cannot use the file: {0}")]
    File(String),
    #[error("the OCR engine failed: {0}")]
    Engine(String),
    #[error("OCR did not end in {} minutes", .0.as_secs().div_ceil(60))]
    TimeLimit(Duration),
    #[error(transparent)]
    Read(#[from] ReadError),
    #[error("a temporary file failed: {0}")]
    Io(#[from] std::io::Error),
}

impl OcrError {
    #[must_use]
    pub fn is_permanent(&self) -> bool {
        matches!(self, Self::File(_) | Self::TimeLimit(_) | Self::Read(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_problem_of_the_file_is_permanent() {
        assert!(OcrError::File("encrypted".to_owned()).is_permanent());
        assert!(OcrError::TimeLimit(Duration::from_hours(1)).is_permanent());
        assert!(OcrError::Read(ReadError::Invalid).is_permanent());
        assert!(!OcrError::Engine("Tesseract stopped".to_owned()).is_permanent());
        assert!(!OcrError::Io(std::io::Error::other("disk full")).is_permanent());
    }

    #[test]
    fn the_time_limit_tells_the_minutes() {
        assert_eq!(
            OcrError::TimeLimit(Duration::from_secs(90)).to_string(),
            "OCR did not end in 2 minutes"
        );
    }
}
