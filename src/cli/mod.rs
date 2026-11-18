pub mod handlers;
pub mod serve;

use std::path::PathBuf;
use clap::{Parser, Subcommand};

pub use handlers::execute_command;

#[derive(Parser, Debug)]
#[command(name = "mymessages", about = "Local-first personal messaging vault and search engine")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the background daemon, webhook receiver, and web server
    Serve {
        #[arg(short, long, env = "MYMESSAGES_CONFIG")]
        config: Option<PathBuf>,

        #[arg(short, long, env = "MYMESSAGES_PORT")]
        port: Option<u16>,

        #[arg(long, env = "MYMESSAGES_HOST")]
        host: Option<String>,

        #[arg(short, long, env = "MYMESSAGES_DATA_DIR")]
        data_dir: Option<PathBuf>,
    },

    /// Search indexed messages from the command line
    Search {
        query: String,

        #[arg(short, long)]
        config: Option<PathBuf>,

        #[arg(short, long)]
        source: Option<String>,

        #[arg(long)]
        chat: Option<String>,

        #[arg(long)]
        has_media: Option<bool>,

        #[arg(long)]
        media_type: Option<String>,

        #[arg(long)]
        from_date: Option<String>,

        #[arg(long)]
        to_date: Option<String>,

        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },

    /// Reindex vault from a designated source of truth
    Reindex {
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Source of truth to read from: 'maildir' or a folder path
        #[arg(long, default_value = "maildir")]
        from: String,

        /// Destination storage target
        #[arg(long, default_value = "db")]
        to: String,
    },

    /// Manage and inspect ingestion adapters
    Adapter {
        #[command(subcommand)]
        command: AdapterCommands,
    },

    /// Manage and inspect configured storage targets
    Storage {
        #[command(subcommand)]
        command: StorageCommands,
    },

    /// Run the Model Context Protocol (MCP) server over stdio for AI assistants
    Mcp {
        #[arg(short, long)]
        config: Option<PathBuf>,

        #[arg(short, long)]
        data_dir: Option<PathBuf>,
    },

    /// Show summary statistics of the vault
    Stats {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AdapterCommands {
    /// List all configured adapters and their status
    List {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Run an adapter to ingest messages
    Run {
        /// Adapter name: 'folder' or 'evolution'
        name: String,

        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Custom path for folder adapter
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum StorageCommands {
    /// List all configured storage targets
    List {
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(["mymessages", "serve", "--port", "9000"]);
        match cli.command {
            Commands::Serve { port, .. } => assert_eq!(port, Some(9000)),
            _ => panic!("Expected serve command"),
        }

        let cli = Cli::parse_from(["mymessages", "storage", "list"]);
        match cli.command {
            Commands::Storage { command: StorageCommands::List { .. } } => (),
            _ => panic!("Expected storage list command"),
        }

        let cli_search = Cli::parse_from([
            "mymessages", "search", "invoice", "--media-type", "document", "--from-date", "2024-01-01",
        ]);
        match cli_search.command {
            Commands::Search { query, media_type, from_date, .. } => {
                assert_eq!(query, "invoice");
                assert_eq!(media_type, Some("document".to_string()));
                assert_eq!(from_date, Some("2024-01-01".to_string()));
            }
            _ => panic!("Expected search command"),
        }
    }
}
