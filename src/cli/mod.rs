use std::path::PathBuf;
use clap::{Parser, Subcommand};

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
        #[arg(short, long)]
        config: Option<PathBuf>,

        #[arg(short, long)]
        port: Option<u16>,

        #[arg(short, long)]
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

        /// Destination to write: 'db', 'maildir', or 'all'
        #[arg(long, default_value = "db")]
        to: String,
    },

    /// Manage and inspect ingestion adapters
    Adapter {
        #[command(subcommand)]
        command: AdapterCommands,
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

        let cli = Cli::parse_from(["mymessages", "reindex", "--from", "maildir", "--to", "db"]);
        match cli.command {
            Commands::Reindex { from, to, .. } => {
                assert_eq!(from, "maildir");
                assert_eq!(to, "db");
            }
            _ => panic!("Expected reindex command"),
        }

        let cli = Cli::parse_from(["mymessages", "adapter", "list"]);
        match cli.command {
            Commands::Adapter { command: AdapterCommands::List { .. } } => (),
            _ => panic!("Expected adapter list command"),
        }
    }
}
