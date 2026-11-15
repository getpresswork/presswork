use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("cannot open the log files in {}", path.display())]
    Files {
        path: PathBuf,
        #[source]
        source: tracing_appender::rolling::InitError,
    },
}
