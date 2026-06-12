use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{fmt, EnvFilter};

mod config;
mod error;
mod state;
mod middleware;
mod routers;
mod dto;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cfg = config::ApiConfig::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&cfg.database_url)
        .await?;

    tracing::info!("Connected to database");

    let app_state = state::AppState::new(pool, cfg.api_bearer_token.clone());

    let app = routers::build_router()
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(app_state);

    let addr = format!("{}:{}", cfg.api_host, cfg.api_port);
    tracing::info!("dict-api listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
