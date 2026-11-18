pub mod adapters;
pub mod api;
pub mod cli;
pub mod config;
pub mod index;
pub mod mcp;
pub mod models;
pub mod storage;

use anyhow::Result;
use clap::Parser;
use crate::cli::{execute_command, Cli};

pub async fn run_app() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    execute_command(cli.command).await
}
