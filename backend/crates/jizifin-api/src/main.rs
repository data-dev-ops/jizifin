//! Jizifin HTTP API. Listens on the port Caddy proxies `/api/*` and `/ws/*` to.

mod auth;
mod config;
mod error;
mod events;
mod health;
mod routes;
mod scheduler;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    if std::env::args().any(|a| a == "--healthcheck") {
        return health::probe(&config.bind).map_err(Into::into);
    }
    init_tracing();
    serve(config).await
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState::open(&config.db_path)?;
    scheduler::spawn(state.clone());
    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    log_listening(&config);
    axum::serve(listener, routes::router(state)).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

fn log_listening(config: &Config) {
    tracing::info!(bind = %config.bind, db = %config.db_path.display(), "jizifin-api listening");
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
