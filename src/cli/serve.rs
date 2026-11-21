use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::adapters::UbackAdapter;
use crate::api::create_router;
use crate::api::state::AppState;
use crate::config::{expand_tilde, AppConfig, CredentialsStore};
use crate::index::{get_vault_stats, is_hash_indexed, Database};

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
    match get_vault_stats(db.conn(), &db_path) {
        Ok(stats) => {
            info!(
                "Vault DB: {} ({:.2} MB) — {} messages across {} chats",
                stats.db_path,
                stats.db_size_bytes as f64 / 1_048_576.0,
                stats.total_messages,
                stats.total_chats
            );
            for s in &stats.sources {
                info!("  Source '{}': {} messages, {} chats", s.source, s.message_count, s.chat_count);
            }
        }
        Err(e) => warn!("Failed to read initial vault stats: {}", e),
    }
    let static_opt = cfg.server.static_dir.clone().filter(|p| p.exists());

    let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
    info!("Starting mymessages server on http://{}", addr);

    let state = AppState::new(db, base_data_dir, cfg);
    spawn_uback_poller(state.clone());
    let app = create_router(state, static_opt);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn spawn_uback_poller(state: AppState) {
    let Some(uback_cfg) = state.config.adapters.uback.clone() else { return };
    if !uback_cfg.enabled || uback_cfg.backup_root.as_os_str().is_empty() {
        return;
    }
    let interval_secs = uback_cfg.poll_interval_seconds.max(5);
    let backup_root = expand_tilde(&uback_cfg.backup_root);
    info!("Starting UBack background poller every {}s on {:?}", interval_secs, backup_root);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            let st = state.clone();
            let root = backup_root.clone();
            let cfg_clone = uback_cfg.clone();
            let res = tokio::task::spawn_blocking(move || {
                let _started = std::time::Instant::now();
                let creds = CredentialsStore::load(cfg_clone.credentials_file.as_deref());
                let adapter = UbackAdapter::new(root, creds, cfg_clone.accounts, cfg_clone.index_media_without_db);
                let summary = adapter.scan_all()?;
                for w in &summary.warnings {
                    warn!("UBack poller warning: {}", w);
                }
                let mut newly_indexed = 0usize;
                for msg in &summary.messages {
                    let hash = msg.compute_content_hash();
                    let already = {
                        let guard = st.db.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
                        is_hash_indexed(guard.conn(), &hash)?
                    };
                    if already {
                        continue;
                    }
                    st.pipeline.write_all(msg, &hash)?;
                    if !msg.id.starts_with("uback_media_") {
                        let guard = st.db.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
                        for att in &msg.attachments {
                            if let Some(ref ext) = att.external_path {
                                let ext_str = ext.to_string_lossy();
                                let _ = guard.conn().execute(
                                    "DELETE FROM messages WHERE id LIKE '%:uback_media_%' AND id IN (SELECT message_id FROM attachments WHERE external_path = ?1)",
                                    [ext_str.as_ref()],
                                );
                            }
                        }
                    }
                    newly_indexed += 1;
                }
                Ok::<usize, anyhow::Error>(newly_indexed)
            }).await;

            match res {
                Ok(Ok(n)) if n > 0 => info!("UBack poller indexed {} new messages", n),
                Ok(Err(e)) => warn!("UBack poller error: {}", e),
                Err(e) => warn!("UBack poller task panicked: {}", e),
                _ => {}
            }
        }
    });
}
