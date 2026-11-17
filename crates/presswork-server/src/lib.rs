pub mod access_tokens;
pub mod accounts;
pub mod chat;
pub mod cli;
mod config;
mod database;
pub mod documents;
mod http;
mod hub;
mod jobs;
pub mod library;
mod limits;
pub mod links;
mod live;
mod routes;
pub mod sessions;
#[cfg(test)]
mod testing;
pub mod work;

use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;

use presswork_ocr::{OcrEngine, OcrMyPdf};

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
    #[error("the library cannot start: {0}")]
    Library(#[source] library::LibraryError),
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

async fn ocr_engine(config: &Config) -> Option<Arc<dyn OcrEngine>> {
    let Some(program) = config.ocr_program().map(str::to_owned) else {
        tracing::info!("OCR is off");
        return None;
    };
    let jobs = std::thread::available_parallelism().map_or(1, NonZeroUsize::get) / jobs::WORKERS;
    let detected = tokio::task::spawn_blocking(move || OcrMyPdf::detect(program))
        .await
        .map_err(|error| error.to_string())
        .and_then(|result| result.map_err(|error| error.to_string()));
    let engine = match detected {
        Ok(engine) => engine.with_jobs(jobs),
        Err(error) => {
            tracing::warn!(%error, "OCR is not available. Scanned pages get no text");
            return None;
        }
    };
    let missing: Vec<String> = config
        .fallback_languages()
        .into_iter()
        .filter(|language| !engine.languages().contains(language))
        .collect();
    if !missing.is_empty() {
        tracing::warn!(?missing, "Tesseract has no data for some OCR languages");
    }
    tracing::info!(
        version = engine.version(),
        languages = ?engine.languages(),
        "OCR uses OCRmyPDF"
    );
    Some(Arc::new(engine))
}

pub async fn run(config: Config) -> Result<(), ServerError> {
    let pool = prepare(&config).await?;
    let mut library = library::Library::open(&config.data).map_err(ServerError::Library)?;
    if let Some(engine) = ocr_engine(&config).await {
        library = library.with_ocr(engine, config.fallback_languages());
    }
    let rebuilt = library
        .rebuild_if_empty(&pool)
        .await
        .map_err(ServerError::Library)?;
    if rebuilt > 0 {
        tracing::info!(works = rebuilt, "The search index was made again");
    }
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(ServerError::Listen)?;
    tracing::info!(address = %config.listen, "Presswork server started");
    let state = AppState::new(pool, library).with_trust_forwarded(config.trust_forwarded);
    state.start_workers();
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await
    .map_err(ServerError::Listen)
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("Presswork server stops");
}
