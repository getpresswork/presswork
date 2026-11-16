pub mod accounts;
pub mod cli;
mod config;
mod database;
pub mod documents;
mod http;
mod hub;
pub mod links;
mod live;
mod routes;
pub mod sessions;
#[cfg(test)]
mod testing;
pub mod work;

pub use config::{Config, ConfigError};
pub use routes::{AppState, router};

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("the configuration has no database URL")]
    NoDatabase,
    #[error("the server cannot connect to the database: {0}")]
    Database(#[source] sqlx::Error),
    #[error("the database migrations failed: {0}")]
    Migration(#[source] sqlx::migrate::MigrateError),
    #[error("the server cannot listen on its address: {0}")]
    Listen(#[source] std::io::Error),
    #[error("the command failed: {0}")]
    Command(#[source] links::LinkError),
}

async fn prepare(config: &Config) -> Result<sqlx::PgPool, ServerError> {
    let url = config
        .database_url
        .as_deref()
        .ok_or(ServerError::NoDatabase)?;
    let pool = database::connect(url)
        .await
        .map_err(ServerError::Database)?;
    database::migrate(&pool)
        .await
        .map_err(ServerError::Migration)?;
    Ok(pool)
}

pub async fn administer(config: &Config, command: cli::Command) -> Result<String, ServerError> {
    let pool = prepare(config).await?;
    cli::execute(&pool, command)
        .await
        .map_err(ServerError::Command)
}

pub async fn run(config: Config) -> Result<(), ServerError> {
    let pool = prepare(&config).await?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(ServerError::Listen)?;
    tracing::info!(address = %config.listen, "Presswork server started");
    axum::serve(listener, router(AppState::new(pool)))
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(ServerError::Listen)
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("Presswork server stops");
}
