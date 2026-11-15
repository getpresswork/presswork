#[derive(Debug, thiserror::Error)]
pub enum ToolchainError {
    #[error("the Typst pin {pin} is not a version requirement: {reason}")]
    InvalidPin { pin: String, reason: String },
    #[error("the list of Typst releases is not valid: {0}")]
    InvalidReleaseList(String),
}
