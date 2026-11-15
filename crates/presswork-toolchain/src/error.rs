#[derive(Debug, thiserror::Error)]
pub enum ToolchainError {
    #[error("the Typst pin {pin} is not a version requirement: {reason}")]
    InvalidPin { pin: String, reason: String },
    #[error("the list of Typst releases is not valid: {0}")]
    InvalidReleaseList(String),
    #[error("the list of known digests has an error on line {0}")]
    InvalidDigestList(usize),
    #[error("the file {0} is not a .zip or a .tar.xz archive")]
    UnsupportedArchive(String),
    #[error("the archive contains an unsafe entry: {0}")]
    UnsafeArchiveEntry(String),
    #[error("the archive is damaged: {0}")]
    BrokenArchive(String),
    #[error("the digest of {file} is {actual}, but the published digest is {published}")]
    DigestMismatch {
        file: String,
        published: String,
        actual: String,
    },
    #[error("the release of Typst {0} does not contain the Typst program")]
    NoProgram(semver::Version),
    #[error("another installation of Typst {0} is in progress")]
    Busy(semver::Version),
    #[error("cannot use {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}
