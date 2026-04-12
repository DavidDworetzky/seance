mod agent;
mod cli;
mod command;
mod config;
mod dashboard;
mod debug;
mod files;
mod ghostty;
mod git;
mod layout;
mod session;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
