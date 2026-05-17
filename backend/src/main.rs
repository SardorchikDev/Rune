//! Rune backend binary.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use rune::{
    agent::memory::MemoryStore,
    config::{shared, RuneConfig},
    core::{db::init_pool, llm::LlmRouter},
    interfaces::api::{build_router, ws::WsEvent},
    interfaces::telegram,
    state::{AppState, WS_CHANNEL_CAPACITY},
    tools::ToolRegistry,
};
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,rune=debug".into()),
        )
        .with_target(true)
        .init();

    let config_path = std::env::var("RUNE_CONFIG").unwrap_or_else(|_| "config.toml".into());
    let cfg =
        RuneConfig::load(&config_path).context(format!("loading config from {config_path}"))?;
    let bind: SocketAddr = format!("{}:{}", cfg.server.host, cfg.server.port)
        .parse()
        .context("invalid server.host / server.port")?;

    let db = init_pool(&cfg.database.url).await?;
    let llm_router = Arc::new(LlmRouter::from_config(&cfg, db.clone())?);
    let memory = MemoryStore::from_config(&cfg, db.clone(), llm_router.clone()).await?;
    let tools = Arc::new(ToolRegistry::from_config(&cfg)?);
    let (ws_broadcast, _) = broadcast::channel::<WsEvent>(WS_CHANNEL_CAPACITY);

    let state = Arc::new(AppState {
        config: shared(cfg.clone()),
        started_at: chrono::Utc::now(),
        db,
        llm_router,
        active_tasks: Arc::new(dashmap::DashMap::new()),
        ws_broadcast,
        memory,
        tools,
    });

    let app = build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context(format!("bind {bind}"))?;
    tracing::info!(addr = %bind, "rune backend listening");

    let server = axum::serve(listener, app);

    let telegram_state = state.clone();
    let telegram_handle = tokio::spawn(async move {
        if let Err(e) = telegram::start(telegram_state).await {
            tracing::error!("telegram bot exited: {e}");
        }
    });

    let result = server.await;
    telegram_handle.abort();
    result.context("axum server exited")?;
    Ok(())
}
