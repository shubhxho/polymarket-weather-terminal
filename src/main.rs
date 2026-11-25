mod api;
mod analytics;
mod gpu;
mod ui;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting Polymarket Weather Terminal");

    let offline_mode = std::env::var("OFFLINE_MODE").is_ok();
    let data_state = Arc::new(RwLock::new(ui::AppData::new(offline_mode)));

    let data_state_clone = Arc::clone(&data_state);
    tokio::spawn(async move {
        if let Err(e) = api::data_refresh_loop(data_state_clone).await {
            tracing::error!("Data refresh loop error: {}", e);
        }
    });

    gpu::run_app(data_state).await?;

    Ok(())
}
