#[derive(Debug, thiserror::Error)]
pub enum ToolchainError {
    #[error("the Typst pin {pin} is not a version requirement: {reason}")]
    InvalidPin { pin: String, reason: String },
    #[error("the list of Typst releases is not valid: {0}")]
    InvalidReleaseList(String),
    #[error("the list of known digests has an error on line {0}")]
    InvalidDigestList(usize),
    #[error("the list of dictionaries has an error on line {0}")]
    InvalidDictionaryList(usize),
    #[error("the file {0} is not a .zip, .tar.gz, or .tar.xz archive")]
    UnsupportedArchive(String),
    #[error("the package name {0} is not valid")]
    InvalidPackage(String),
    #[error("the package {0} is not in the package index")]
    PackageNotFound(String),
    #[error("the package {package} does not exist in this version; the latest version is {latest}")]
    PackageVersionNotFound { package: String, latest: String },
    #[error("the package index is not valid: {0}")]
    InvalidPackageIndex(String),
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
    #[error("Typst {0} has no release file for this platform")]
    NoFile(semver::Version),
    #[error("Typst {0} has no published digest, so it cannot be checked")]
    NoDigest(semver::Version),
    #[error("the installation stopped before it was complete: {0}")]
    Interrupted(String),
    #[error("the network request failed: {0}")]
    Network(String),
    #[error("the server answered with the HTTP status {status}")]
    Http { status: u16 },
    #[error("cannot use {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}
