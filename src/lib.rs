pub mod adapters;
pub mod api;
pub mod cli;
pub mod config;
pub mod index;
pub mod models;
pub mod storage;

use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

use crate::api::create_router;
use crate::api::state::AppState;
use crate::cli::{AdapterCommands, Cli, Commands};
use crate::config::AppConfig;
use crate::index::{reindex_from_folder, reindex_from_maildir, search_messages, Database, SearchQuery};
use crate::storage::MaildirStorage;

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
            if let Some(d) = data_dir { cfg.storage.data_dir = d; }

            fs::create_dir_all(&cfg.storage.data_dir)
                .with_context(|| format!("Failed to create data dir {:?}", cfg.storage.data_dir))?;

            let db_path = cfg.storage.data_dir.join("messages.sqlite3");
            let db = Database::open(&db_path)?;
            let static_opt = cfg.server.static_dir.clone().filter(|p| p.exists());

            let addr = format!("{}:{}", cfg.server.host, cfg.server.port);
            info!("Starting mymessages server on http://{}", addr);

            let state = AppState::new(db, cfg.storage.data_dir.clone(), cfg);
            let app = create_router(state, static_opt);

            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Search { query, config, source, chat, has_media, limit } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let db_path = cfg.storage.data_dir.join("messages.sqlite3");
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
            fs::create_dir_all(&cfg.storage.data_dir)?;
            let db_path = cfg.storage.data_dir.join("messages.sqlite3");
            let mut db = Database::open(&db_path)?;
            let storage = MaildirStorage::new(&cfg.storage.data_dir);

            println!("Starting reindex from '{}' to '{}'...", from, to);
            let enable_maildir = to == "maildir" || to == "all" || cfg.storage.enable_maildir;
            let enable_db = to == "db" || to == "all" || cfg.storage.enable_db;

            let report = if from == "maildir" {
                reindex_from_maildir(db.conn_mut(), &cfg.storage.data_dir)?
            } else {
                let path = PathBuf::from(&from);
                reindex_from_folder(db.conn_mut(), &storage, &path, enable_maildir, enable_db)?
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
                fs::create_dir_all(&cfg.storage.data_dir)?;
                let db_path = cfg.storage.data_dir.join("messages.sqlite3");
                let mut db = Database::open(&db_path)?;
                let storage = MaildirStorage::new(&cfg.storage.data_dir);

                if name == "folder" {
                    let folder_path = path.or_else(|| cfg.adapters.folder.as_ref().map(|f| f.folder_path.clone()))
                        .unwrap_or_else(|| PathBuf::from("./test_messages"));
                    println!("Running folder adapter on {:?}...", folder_path);
                    let report = reindex_from_folder(
                        db.conn_mut(),
                        &storage,
                        &folder_path,
                        cfg.storage.enable_maildir,
                        cfg.storage.enable_db,
                    )?;
                    println!("Ingested {} messages ({} duplicates skipped).", report.newly_indexed, report.duplicates_skipped);
                } else {
                    println!("Adapter '{}' is an event-driven webhook adapter. Start the daemon with 'serve' to receive webhooks.", name);
                }
            }
        },

        Commands::Stats { config } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let db_path = cfg.storage.data_dir.join("messages.sqlite3");
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
