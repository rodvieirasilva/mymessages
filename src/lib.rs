pub mod adapters;
pub mod api;
pub mod cli;
pub mod config;
pub mod index;
pub mod models;
pub mod storage;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use crate::api::create_router;
use crate::api::state::AppState;
use crate::cli::{AdapterCommands, Cli, Commands, StorageCommands};
use crate::config::AppConfig;
use crate::index::{reindex_from_folder, reindex_from_maildir, search_messages, Database, SearchQuery};
use crate::storage::{MaildirStorage, StoragePipeline};

pub async fn run_app() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config, port, data_dir } => {
            let mut cfg = AppConfig::load_or_default(config.as_deref());
            if let Some(p) = port { cfg.server.port = p; }
            let base_data_dir = data_dir.unwrap_or_else(|| PathBuf::from("./data"));

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
        }

        Commands::Search { query, config, source, chat, has_media, limit } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let db_path = cfg.get_storage_path("sqlite")
                .unwrap_or_else(|| PathBuf::from("./data/messages.sqlite3"));
            if !db_path.exists() {
                println!("No database found at {:?}", db_path);
                return Ok(());
            }

            let db = Database::open(&db_path)?;
            let q = SearchQuery {
                query,
                source,
                chat_id: chat,
                has_media,
                limit: Some(limit),
                ..Default::default()
            };

            let results = search_messages(db.conn(), &q)?;
            println!("Found {} results:", results.len());
            for r in results {
                let ts = chrono::DateTime::from_timestamp(r.timestamp, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default();
                let sender = r.sender_name.as_deref().unwrap_or("Unknown");
                let chat_name = r.conversation_name.as_deref().unwrap_or("Chat");
                println!("[{}] {} in {}: {}", ts, sender, chat_name, r.body.as_deref().unwrap_or(""));
                if let Some(snip) = r.snippet {
                    println!("   Match: {}", snip);
                }
                for att in &r.attachments {
                    println!("   [Attachment: {} ({})]", att.filename.as_deref().unwrap_or("file"), att.media_type);
                }
            }
        }

        Commands::Reindex { config, from, to } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let base_data_dir = PathBuf::from("./data");
            fs::create_dir_all(&base_data_dir)?;

            let db_path = cfg.get_storage_path("sqlite")
                .unwrap_or_else(|| base_data_dir.join("messages.sqlite3"));
            let mail_path = cfg.get_storage_path("maildir")
                .unwrap_or_else(|| base_data_dir.join("mail"));

            let mut db = Database::open(&db_path)?;
            println!("Starting reindex from '{}' to '{}'...", from, to);

            let report = if from == "maildir" {
                reindex_from_maildir(db.conn_mut(), &mail_path)?
            } else {
                let path = PathBuf::from(&from);
                let db_arc = Arc::new(Mutex::new(db));
                let storage_arc = Arc::new(MaildirStorage::new(&mail_path));
                let pipeline = StoragePipeline::from_config(&cfg, db_arc, storage_arc);
                reindex_from_folder(&pipeline, &path)?
            };

            println!("=== Reindex Complete ===");
            println!("Total found:          {}", report.total_found);
            println!("Newly indexed:        {}", report.newly_indexed);
            println!("Duplicates skipped:   {}", report.duplicates_skipped);
        }

        Commands::Adapter { command } => match command {
            AdapterCommands::List { config } => {
                let cfg = AppConfig::load_or_default(config.as_deref());
                println!("=== Configured Ingestion Adapters ===");
                println!("1. Evolution API (WhatsApp):");
                println!("   - Enabled:      {}", cfg.adapters.evolution.enabled);
                println!("   - Webhook Path: {}", cfg.adapters.evolution.webhook_path);
                if let Some(folder_cfg) = &cfg.adapters.folder {
                    println!("2. Folder Adapter:");
                    println!("   - Enabled:      {}", folder_cfg.enabled);
                    println!("   - Folder Path:  {:?}", folder_cfg.folder_path);
                    println!("   - Pattern:      {}", folder_cfg.file_pattern);
                }
            }
            AdapterCommands::Run { name, config, path } => {
                let cfg = AppConfig::load_or_default(config.as_deref());
                let base_data_dir = PathBuf::from("./data");
                fs::create_dir_all(&base_data_dir)?;
                let db_path = cfg.get_storage_path("sqlite").unwrap_or_else(|| base_data_dir.join("messages.sqlite3"));
                let mail_path = cfg.get_storage_path("maildir").unwrap_or_else(|| base_data_dir.join("mail"));

                let db = Database::open(&db_path)?;
                let db_arc = Arc::new(Mutex::new(db));
                let storage_arc = Arc::new(MaildirStorage::new(&mail_path));
                let pipeline = StoragePipeline::from_config(&cfg, db_arc, storage_arc);

                if name == "folder" {
                    let folder_path = path.or_else(|| cfg.adapters.folder.as_ref().map(|f| f.folder_path.clone()))
                        .unwrap_or_else(|| PathBuf::from("./test_messages"));
                    println!("Running folder adapter on {:?}...", folder_path);
                    let report = reindex_from_folder(&pipeline, &folder_path)?;
                    println!("Ingested {} messages ({} skipped).", report.newly_indexed, report.duplicates_skipped);
                } else {
                    println!("Adapter '{}' is an event-driven webhook adapter. Start with 'serve' to receive webhooks.", name);
                }
            }
        },

        Commands::Storage { command } => match command {
            StorageCommands::List { config } => {
                let cfg = AppConfig::load_or_default(config.as_deref());
                println!("=== Configured Storage Sinks ===");
                for (idx, entry) in cfg.storages.iter().enumerate() {
                    println!("{}. Name: '{}' (Type: {:?})", idx + 1, entry.name, entry.storage_type);
                    println!("   - Enabled: {}", entry.enabled);
                    println!("   - Path:    {:?}", entry.path);
                }
            }
        },

        Commands::Stats { config } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let db_path = cfg.get_storage_path("sqlite").unwrap_or_else(|| PathBuf::from("./data/messages.sqlite3"));
            if !db_path.exists() {
                println!("No database found at {:?}", db_path);
                return Ok(());
            }
            let db = Database::open(&db_path)?;
            let total_msgs: i64 = db.conn().query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?;
            let total_chats: i64 = db.conn().query_row("SELECT count(*) FROM conversations", [], |r| r.get(0))?;
            let total_atts: i64 = db.conn().query_row("SELECT count(*) FROM attachments", [], |r| r.get(0))?;

            println!("=== mymessages Vault Statistics ===");
            println!("Total Messages:    {}", total_msgs);
            println!("Total Chats:       {}", total_chats);
            println!("Total Attachments: {}", total_atts);
        }
    }

    Ok(())
}
