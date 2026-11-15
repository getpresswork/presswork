use std::path::PathBuf;

use directories::ProjectDirs;
use presswork_log::{DEFAULT_FILTER, LogConfig, Logging};

const FILTER_VARIABLE: &str = "PRESSWORK_LOG";

pub(crate) fn start() -> Option<Logging> {
    let filter = std::env::var(FILTER_VARIABLE).ok();
    let mut problems = Vec::new();
    for config in configs(log_directory(), filter) {
        match presswork_log::init(&config) {
            Ok(logging) => {
                for problem in &problems {
                    tracing::warn!(error = %problem, "Logging uses a fallback configuration");
                }
                return Some(logging);
            }
            Err(problem) => problems.push(problem),
        }
    }
    None
}

pub(crate) fn configs(directory: Option<PathBuf>, filter: Option<String>) -> Vec<LogConfig> {
    let filter = filter.unwrap_or_else(|| DEFAULT_FILTER.to_owned());
    vec![
        LogConfig {
            directory,
            filter: filter.clone(),
            ..LogConfig::default()
        },
        LogConfig {
            directory: None,
            filter,
            ..LogConfig::default()
        },
        LogConfig::default(),
    ]
}

pub(crate) fn log_directory() -> Option<PathBuf> {
    ProjectDirs::from("io.github", "getpresswork", "Presswork")
        .map(|directories| directories.data_local_dir().join("logs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_config_uses_the_log_directory_and_the_filter() {
        let configs = configs(Some(PathBuf::from("logs")), Some("debug".to_owned()));

        assert_eq!(configs[0].directory, Some(PathBuf::from("logs")));
        assert_eq!(configs[0].filter, "debug");
    }

    #[test]
    fn fallback_configs_drop_the_files_and_then_the_custom_filter() {
        let configs = configs(Some(PathBuf::from("logs")), Some("debug".to_owned()));

        assert_eq!(configs[1].directory, None);
        assert_eq!(configs[1].filter, "debug");
        assert_eq!(configs[2].directory, None);
        assert_eq!(configs[2].filter, DEFAULT_FILTER);
    }

    #[test]
    fn log_directory_is_a_logs_folder() {
        assert!(log_directory().is_some_and(|path| path.ends_with("logs")));
    }
}
