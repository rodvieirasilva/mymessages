use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use tracing::info;

use crate::api::create_router;
use crate::api::state::AppState;
use crate::config::AppConfig;
use crate::index::Database;

pub async fn run_serve_command(
    config: Option<PathBuf>,
    port: Option<u16>,
    host: Option<String>,
    data_dir: Option<PathBuf>,
) -> Result<()> {
    let mut cfg = AppConfig::load_or_default(config.as_deref());
    if let Some(h) = host { cfg.server.host = h; }
    if let Some(p) = port { cfg.server.port = p; }
    let base_data_dir = data_dir.unwrap_or_else(|| {
        std::env::var("MYMESSAGES_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"))
    });

    fs::create_dir_all(&base_data_dir)
        .with_context(|| format!("Failed to create data dir {:?}", base_data_dir))?;

    let db_path = cfg.get_storage_path("sqlite")
        .unwrap_or_else(|| base_data_dir.join("messages.sqlite3"));
    let db = Database::open(&db_path)?;
    let static_opt = cfg.server.static_dir.clone().filter(|p| p.exists());

    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    info!("Starting mymessages server on http://{}", addr);

    let state = AppState::new(db, base_data_dir, cfg);
    let app = create_router(state, static_opt);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
