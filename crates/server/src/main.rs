//! YellowRakk server entry point.

use std::net::SocketAddr;
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use tracing_subscriber::EnvFilter;
use yellowrakk_products::fetch::PageFetcher;
use yellowrakk_server::{AppState, router};

#[derive(Debug, thiserror::Error)]
enum StartupError {
    #[error("invalid {name}: {reason}")]
    Config { name: &'static str, reason: String },
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migrations: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("http client: {0}")]
    Fetcher(#[from] yellowrakk_products::fetch::FetchError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

fn env_or(name: &'static str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let database_url = env_or("DATABASE_URL", "sqlite://data/yellowrakk.db");
    let bind_addr = env_or("BIND_ADDR", "127.0.0.1:8080");
    let bind_addr = SocketAddr::from_str(&bind_addr).map_err(|e| StartupError::Config {
        name: "BIND_ADDR",
        reason: e.to_string(),
    })?;

    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;

    let state = AppState::new(pool, PageFetcher::new()?);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!("listening on http://{bind_addr}");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            // If the signal handler cannot be installed, just run until killed.
            if tokio::signal::ctrl_c().await.is_err() {
                std::future::pending::<()>().await;
            }
        })
        .await?;
    Ok(())
}
