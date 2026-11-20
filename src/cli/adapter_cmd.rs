use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::Result;

use crate::adapters::UbackAdapter;
use crate::cli::AdapterCommands;
use crate::config::{expand_tilde, AppConfig, CredentialsStore};
use crate::index::{is_hash_indexed, reindex_from_folder, Database};
use crate::storage::{MaildirStorage, StoragePipeline};

pub fn execute_adapter_command(command: AdapterCommands) -> Result<()> {
    match command {
        AdapterCommands::List { config } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            println!("=== Configured Ingestion Adapters ===");
            println!("1. Evolution API (WhatsApp):");
            println!("   - Enabled:      {}", cfg.adapters.evolution.enabled);
            println!("   - Webhook Path: {}", cfg.adapters.evolution.webhook_path);
            println!("2. WAHA (WhatsApp HTTP API):");
            println!("   - Enabled:      {}", cfg.adapters.waha.enabled);
            println!("   - Webhook Path: {}", cfg.adapters.waha.webhook_path);
            println!("   - API URL:      {}", cfg.adapters.waha.api_url);
            if let Some(folder_cfg) = &cfg.adapters.folder {
                println!("3. Folder Adapter:");
                println!("   - Enabled:      {}", folder_cfg.enabled);
                println!("   - Folder Path:  {:?}", folder_cfg.folder_path);
                println!("   - Pattern:      {}", folder_cfg.file_pattern);
            }
            if let Some(uback_cfg) = &cfg.adapters.uback {
                let creds = CredentialsStore::load(uback_cfg.credentials_file.as_deref());
                println!("4. UBack Adapter (WhatsApp Media & Crypt15 Backups):");
                println!("   - Enabled:            {}", uback_cfg.enabled);
                println!("   - Backup Root:        {:?}", uback_cfg.backup_root);
                println!("   - Accounts Filter:    {:?}", uback_cfg.accounts);
                println!("   - Keys Configured:    {:?}", creds.whatsapp_accounts());
            }
        }
        AdapterCommands::Run {
            name,
            config,
            path,
            credentials,
            account,
            index_media_without_db,
        } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let base_data_dir = PathBuf::from("./data");
            fs::create_dir_all(&base_data_dir)?;
            let db_path = cfg.get_storage_path("sqlite").unwrap_or_else(|| base_data_dir.join("messages.sqlite3"));
            let mail_path = cfg.get_storage_path("maildir").unwrap_or_else(|| base_data_dir.join("mail"));

            let db = Database::open(&db_path)?;
            let db_arc = Arc::new(Mutex::new(db));
            let storage_arc = Arc::new(MaildirStorage::new(&mail_path));
            let pipeline = StoragePipeline::from_config(&cfg, db_arc.clone(), storage_arc);

            if name == "folder" {
                let folder_path = path
                    .or_else(|| cfg.adapters.folder.as_ref().map(|f| f.folder_path.clone()))
                    .unwrap_or_else(|| PathBuf::from("./tests/test_messages"));
                println!("Running folder adapter on {:?}...", folder_path);
                let report = reindex_from_folder(&pipeline, &folder_path)?;
                println!("Ingested {} messages ({} skipped).", report.newly_indexed, report.duplicates_skipped);
            } else if name == "uback" {
                let uback_cfg = cfg.adapters.uback.clone().unwrap_or_default();
                let backup_root = path
                    .or_else(|| {
                        if !uback_cfg.backup_root.as_os_str().is_empty() {
                            Some(uback_cfg.backup_root.clone())
                        } else {
                            std::env::var_os("MYMESSAGES_UBACK_ROOT").map(PathBuf::from)
                        }
                    })
                    .map(|p| expand_tilde(&p))
                    .ok_or_else(|| anyhow::anyhow!("UBack backup_root not set (pass --path, set MYMESSAGES_UBACK_ROOT, or configure [adapters.uback])"))?;

                let creds_path = credentials.or(uback_cfg.credentials_file);
                let creds = CredentialsStore::load(creds_path.as_deref());
                let acct_filter = if let Some(a) = account { vec![a] } else { uback_cfg.accounts };
                let media_fallback = index_media_without_db || uback_cfg.index_media_without_db;

                println!("Running UBack adapter on {:?} (zero-duplication media mode)...", backup_root);
                let adapter = UbackAdapter::new(backup_root, creds, acct_filter, media_fallback);
                let summary = adapter.scan_all()?;

                for w in &summary.warnings {
                    eprintln!("Warning: {}", w);
                }

                let mut newly_indexed = 0usize;
                let mut skipped = 0usize;
                for msg in &summary.messages {
                    let hash = msg.compute_content_hash();
                    let already = {
                        let guard = db_arc.lock().map_err(|e| anyhow::anyhow!("DB lock error: {}", e))?;
                        is_hash_indexed(guard.conn(), &hash)?
                    };
                    if already {
                        skipped += 1;
                        continue;
                    }
                    pipeline.write_all(msg, &hash)?;
                    if !msg.id.starts_with("uback_media_") {
                        let guard = db_arc.lock().map_err(|e| anyhow::anyhow!("DB lock error: {}", e))?;
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

                println!(
                    "UBack sync complete: accounts={:?}, indexed={}, duplicates_skipped={}",
                    summary.accounts_scanned, newly_indexed, skipped
                );
            } else {
                println!("Adapter '{}' is an event-driven webhook adapter. Start with 'serve' to receive webhooks.", name);
            }
        }
    }
    Ok(())
}
