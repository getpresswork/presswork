use std::path::Path;

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::LogError;

const DAYS_KEPT: usize = 14;

pub fn open_log_files(directory: &Path) -> Result<(NonBlocking, WorkerGuard), LogError> {
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("presswork")
        .filename_suffix("log")
        .max_log_files(DAYS_KEPT)
        .build(directory)
        .map_err(|source| LogError::Files {
            path: directory.to_path_buf(),
            source,
        })?;
    Ok(tracing_appender::non_blocking(appender))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LogLayer, LogStore};
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn writes_each_entry_as_a_json_line_in_a_log_file() {
        let directory = tempfile::tempdir().unwrap();
        let logs = directory.path().join("logs");
        let (writer, guard) = open_log_files(&logs).unwrap();
        let layer = LogLayer::new(LogStore::with_capacity(10)).with_file(writer);
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(project = "book", "compiled");
        });
        drop(guard);

        let files: Vec<_> = std::fs::read_dir(&logs).unwrap().flatten().collect();
        assert_eq!(files.len(), 1);
        let path = files[0].path();
        let name = files[0].file_name().to_string_lossy().into_owned();
        assert!(name.starts_with("presswork"), "{name}");
        assert!(path.extension().is_some_and(|extension| extension == "log"));
        let content = std::fs::read_to_string(files[0].path()).unwrap();
        let line: serde_json::Value = serde_json::from_str(content.trim_end()).unwrap();
        assert_eq!(line["message"], "compiled");
        assert_eq!(line["fields"]["project"], "book");
    }
}
