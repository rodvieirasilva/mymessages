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
    if let Some(tmp) = std::env::var_os("MYMESSAGES_TMP_DIR").map(PathBuf::from).or_else(|| cfg.server.tmp_dir.clone()) {
        let expanded = expand_tilde(&tmp);
        fs::create_dir_all(&expanded)
            .with_context(|| format!("Failed to create tmp_dir {:?}", expanded))?;
        unsafe { std::env::set_var("TMPDIR", &expanded); }
        info!("Using TMPDIR: {:?}", expanded);
    }
    let base_data_dir = data_dir.unwrap_or_else(|| {
        std::env::var("MYMESSAGES_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"))
    });

    fs::create_dir_all(&base_data_dir)
        .with_context(|| format!("Failed to create data dir {:?}", base_data_dir))?;

    let db_path = cfg.get_storage_path("sqlite")
        .unwrap_or_else(|| base_data_dir.join("messages.sqlite3"));
    let db = open_or_recover_db(&db_path)?;
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
    let shutdown_state = state.clone();
    spawn_uback_poller(state.clone());
    let app = create_router(state, static_opt);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutting down cleanly: checkpointing SQLite DB and cleaning temp files...");
    if let Ok(guard) = shutdown_state.db.lock() {
        let _ = guard.conn().execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA optimize;");
    }
    cleanup_temp_files();
    info!("Shutdown complete.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn cleanup_temp_files() {
    if let Ok(entries) = fs::read_dir(std::env::temp_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("mymessages_dec_") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn open_or_recover_db(db_path: &std::path::Path) -> Result<Database> {
    let try_open = || -> Result<Database> {
        let db = Database::open(db_path)?;
        db.check_integrity()?;
        Ok(db)
    };
    match try_open() {
        Ok(db) => Ok(db),
        Err(e) => {
            let msg = e.to_string();
            if db_path.exists()
                && (msg.contains("malformed")
                    || msg.contains("corrupt")
                    || msg.contains("not a database")
                    || msg.contains("integrity check failed"))
            {
                let ts = chrono::Utc::now().timestamp();
                let backup = db_path.with_extension(format!("sqlite3.corrupt.{}.bak", ts));
                warn!(
                    "Database {:?} is corrupted ({}). Backing up to {:?} and recreating fresh DB...",
                    db_path, msg, backup
                );
                let _ = fs::rename(db_path, &backup);
                let base = db_path.to_string_lossy();
                let _ = fs::remove_file(format!("{}-wal", base));
                let _ = fs::remove_file(format!("{}-shm", base));
                Database::open(db_path)
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_or_recover_corrupt_db() {
        let dir = std::env::temp_dir().join("mymessages_test_corrupt_recover");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("messages.sqlite3");
        fs::write(&db_path, b"not a valid sqlite database file at all!!!").unwrap();

        let db = open_or_recover_db(&db_path).unwrap();
        assert!(db.check_integrity().is_ok());
        let has_backup = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt."));
        assert!(has_backup);

        let _ = fs::remove_dir_all(&dir);
    }
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
                if let Ok(mut ing) = st.ingestion.lock() {
                    ing.active = true;
                    ing.phase = "scanning".to_string();
                    ing.scanned_total = 0;
                    ing.processed = 0;
                    ing.newly_indexed = 0;
                }
                let _started = std::time::Instant::now();
                let creds = CredentialsStore::load(cfg_clone.credentials_file.as_deref());
                let adapter = UbackAdapter::new(root, creds, cfg_clone.accounts, cfg_clone.index_media_without_db);
                let summary = adapter.scan_all()?;
                for w in &summary.warnings {
                    warn!("UBack poller warning: {}", w);
                }
                let total = summary.messages.len();
                if let Ok(mut ing) = st.ingestion.lock() {
                    ing.phase = "indexing".to_string();
                    ing.scanned_total = total;
                }
                let mut newly_indexed = 0usize;
                for (idx, msg) in summary.messages.iter().enumerate() {
                    let hash = msg.compute_content_hash();
                    let already = {
                        let guard = st.db.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
                        is_hash_indexed(guard.conn(), &hash)?
                    };
                    if already {
                        if idx % 100 == 0 {
                            if let Ok(mut ing) = st.ingestion.lock() {
                                ing.processed = idx + 1;
                            }
                        }
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
                    if newly_indexed % 25 == 0 || idx + 1 == total {
                        if let Ok(mut ing) = st.ingestion.lock() {
                            ing.processed = idx + 1;
                            ing.newly_indexed = newly_indexed;
                        }
                    }
                }
                if let Ok(mut ing) = st.ingestion.lock() {
                    ing.active = false;
                    ing.phase = "idle".to_string();
                    ing.processed = total;
                    ing.newly_indexed = newly_indexed;
                    ing.last_completed_at = Some(chrono::Utc::now().timestamp());
                }
                Ok::<usize, anyhow::Error>(newly_indexed)
            }).await;

            if let Ok(mut ing) = state.ingestion.lock() {
                if ing.active {
                    ing.active = false;
                    ing.phase = "idle".to_string();
                }
            }

            match res {
                Ok(Ok(n)) if n > 0 => info!("UBack poller indexed {} new messages", n),
                Ok(Err(e)) => warn!("UBack poller error: {}", e),
                Err(e) => warn!("UBack poller task panicked: {}", e),
                _ => {}
            }
        }
    });
}
