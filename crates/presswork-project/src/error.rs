use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("the folder {0} has no presswork.toml manifest")]
    NoManifest(PathBuf),
    #[error("the manifest is not valid: {0}")]
    InvalidManifest(String),
    #[error("the main file {0} is not in the project folder")]
    MainOutside(PathBuf),
    #[error("the main file {0} does not exist")]
    MainMissing(PathBuf),
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
