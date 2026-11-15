use std::path::PathBuf;

use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry, reload};

use crate::{LogError, LogLayer, LogStore, open_log_files};

pub const DEFAULT_FILTER: &str =
    "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,iced_wgpu=warn,cosmic_text=warn";

const DEFAULT_CAPACITY: usize = 10_000;

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub directory: Option<PathBuf>,
    pub filter: String,
    pub capacity: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            directory: None,
            filter: DEFAULT_FILTER.to_owned(),
            capacity: DEFAULT_CAPACITY,
        }
    }
}

#[derive(Debug)]
pub struct Logging {
    pub store: LogStore,
    pub levels: LevelControl,
    _files: Option<WorkerGuard>,
}

#[derive(Debug, Clone)]
pub struct LevelControl(reload::Handle<EnvFilter, Registry>);

impl LevelControl {
    pub fn set(&self, directives: &str) -> Result<(), LogError> {
        let filter = parse_filter(directives)?;
        self.0.reload(filter).map_err(LogError::Reload)
    }
}

pub fn build(
    config: &LogConfig,
) -> Result<(impl Subscriber + Send + Sync + 'static, Logging), LogError> {
    let store = LogStore::with_capacity(config.capacity);
    let (filter, handle) = reload::Layer::new(parse_filter(&config.filter)?);
    let mut layer = LogLayer::new(store.clone());
    let mut files = None;
    if let Some(directory) = &config.directory {
        let (writer, guard) = open_log_files(directory)?;
        layer = layer.with_file(writer);
        files = Some(guard);
    }
    let subscriber = tracing_subscriber::registry().with(filter).with(layer);
    let logging = Logging {
        store,
        levels: LevelControl(handle),
        _files: files,
    };
    Ok((subscriber, logging))
}

pub fn init(config: &LogConfig) -> Result<Logging, LogError> {
    let (subscriber, logging) = build(config)?;
    subscriber.try_init().map_err(LogError::Init)?;
    Ok(logging)
}

fn parse_filter(directives: &str) -> Result<EnvFilter, LogError> {
    EnvFilter::try_new(directives).map_err(|source| LogError::Filter {
        directives: directives.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(store: &LogStore) -> Vec<String> {
        store
            .after(0)
            .into_iter()
            .map(|entry| entry.message)
            .collect()
    }

    #[test]
    fn default_config_keeps_information_and_drops_debug_entries() {
        let (subscriber, logging) = build(&LogConfig::default()).unwrap();

        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("hidden");
            tracing::info!("shown");
        });

        assert_eq!(messages(&logging.store), ["shown"]);
    }

    #[test]
    fn levels_can_change_while_the_program_runs() {
        let (subscriber, logging) = build(&LogConfig::default()).unwrap();

        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("before");
            logging.levels.set("debug").unwrap();
            tracing::debug!("after");
        });

        assert_eq!(messages(&logging.store), ["after"]);
    }

    #[test]
    fn an_invalid_filter_gives_an_error() {
        let config = LogConfig {
            filter: "info,=[".to_owned(),
            ..LogConfig::default()
        };

        assert!(matches!(build(&config), Err(LogError::Filter { .. })));
    }
}
