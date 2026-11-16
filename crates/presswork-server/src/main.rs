use std::process::ExitCode;

use presswork_server::{Config, run};
use tracing_subscriber::EnvFilter;

const CONFIG_VARIABLE: &str = "PRESSWORK_CONFIG";

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let text = match std::env::var_os(CONFIG_VARIABLE) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                tracing::error!(%error, path = %path.to_string_lossy(), "The configuration file cannot be read");
                return ExitCode::FAILURE;
            }
        },
        None => String::new(),
    };
    let config = match Config::parse(&text, |name| std::env::var(name).ok()) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(%error, "The configuration is not valid");
            return ExitCode::FAILURE;
        }
    };
    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "The server stopped");
            ExitCode::FAILURE
        }
    }
}
