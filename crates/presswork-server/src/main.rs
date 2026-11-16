use std::process::ExitCode;

use clap::Parser;
use presswork_server::cli::{Cli, Command};
use presswork_server::{Config, administer, run};
use tracing_subscriber::EnvFilter;

const CONFIG_VARIABLE: &str = "PRESSWORK_CONFIG";

#[tokio::main]
async fn main() -> ExitCode {
    let command = Cli::parse().command();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if command == Command::Serve {
            "info"
        } else {
            "warn"
        })
    });
    if command == Command::Serve {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    }
    let Some(config) = load_config() else {
        return ExitCode::FAILURE;
    };
    let result = match command {
        Command::Serve => run(config).await,
        command => administer(&config, command).await.map(|output| {
            print!("{output}");
        }),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "The command failed");
            ExitCode::FAILURE
        }
    }
}

fn load_config() -> Option<Config> {
    let text = match std::env::var_os(CONFIG_VARIABLE) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                tracing::error!(%error, path = %path.to_string_lossy(), "The configuration file cannot be read");
                return None;
            }
        },
        None => String::new(),
    };
    match Config::parse(&text, |name| std::env::var(name).ok()) {
        Ok(config) => Some(config),
        Err(error) => {
            tracing::error!(%error, "The configuration is not valid");
            None
        }
    }
}
