use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::Result;

use crate::cli::adapter_cmd::execute_adapter_command;
use crate::cli::{Commands, StorageCommands};
use crate::config::AppConfig;
use crate::index::{list_accounts, reindex_from_folder, reindex_from_maildir, search_compiled, Database, SearchQuery};
use crate::mcp::run_mcp_server;
use crate::storage::{MaildirStorage, StoragePipeline};

pub async fn execute_command(command: Commands) -> Result<()> {
    match command {
        Commands::Serve { config, port, host, data_dir } => {
            crate::cli::serve::run_serve_command(config, port, host, data_dir).await?;
        }

        Commands::Search {
            query,
            config,
            source,
            account,
            chat,
            has_media,
            media_type,
            from_date,
            to_date,
            limit,
        } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let db_path = cfg.get_storage_path("sqlite")
                .unwrap_or_else(|| PathBuf::from("./data/messages.sqlite3"));
            if !db_path.exists() {
                println!("No database found at {:?}", db_path);
                return Ok(());
            }

            let db = Database::open(&db_path)?;
            let mut compiled = SearchQuery {
                query,
                source,
                account,
                chat_id: chat,
                has_media,
                media_type,
                limit: Some(limit),
                ..Default::default()
            }.to_compiled();

            if let Some(fd) = from_date {
                compiled.from_timestamp = crate::index::lucene::parse_flexible_date(&fd);
            }
            if let Some(td) = to_date {
                compiled.to_timestamp = crate::index::lucene::parse_flexible_date(&td);
            }

            let results = search_compiled(db.conn(), &compiled)?;
            println!("Found {} results:", results.len());
            for r in results {
                let ts = chrono::DateTime::from_timestamp(r.timestamp, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default();
                let sender = r.sender_name.as_deref().unwrap_or("Unknown");
                let chat_name = r.conversation_name.as_deref().unwrap_or("Chat");
                println!("[{}][{}] {} in {}: {}", ts, r.account, sender, chat_name, r.body.as_deref().unwrap_or(""));
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

        Commands::Adapter { command } => {
            execute_adapter_command(command)?;
        }

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
            let accounts = list_accounts(db.conn())?;

            println!("=== mymessages Vault Statistics ===");
            println!("Total Messages:    {}", total_msgs);
            println!("Total Chats:       {}", total_chats);
            println!("Total Attachments: {}", total_atts);
            println!("Accounts ({}):", accounts.len());
            for a in accounts {
                println!("  - {} ({} msgs, {} chats, sources: {:?})", a.id, a.message_count, a.chat_count, a.sources);
            }
        }

        Commands::Mcp { config, data_dir } => {
            let cfg = AppConfig::load_or_default(config.as_deref());
            let base_data_dir = data_dir.unwrap_or_else(|| PathBuf::from("./data"));
            let db_path = cfg
                .get_storage_path("sqlite")
                .unwrap_or_else(|| base_data_dir.join("messages.sqlite3"));
            let db = Database::open(&db_path)?;
            run_mcp_server(db).await?;
        }
    }

    Ok(())
}
