#[derive(Debug, thiserror::Error)]
pub enum ToolchainError {
    #[error("the Typst pin {pin} is not a version requirement: {reason}")]
    InvalidPin { pin: String, reason: String },
}
