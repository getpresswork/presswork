use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("cannot open the log files in {}", path.display())]
    Files {
        path: PathBuf,
        #[source]
        source: tracing_appender::rolling::InitError,
    },
    #[error("the log filter {directives:?} is not valid")]
    Filter {
        directives: String,
        #[source]
        source: tracing_subscriber::filter::ParseError,
    },
    #[error("cannot change the log filter")]
    Reload(#[source] tracing_subscriber::reload::Error),
    #[error("logging is already set up")]
    Init(#[source] tracing_subscriber::util::TryInitError),
}
