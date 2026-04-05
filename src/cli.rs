use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::command;

#[derive(Parser)]
#[command(
    name = "seance",
    about = "A gathering of spirits — orchestrate parallel AI coding sessions",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a worktree with an agent group in the next available quadrant
    Add(command::add::AddArgs),

    /// Remove a worktree and its agents (no merge)
    Remove(command::remove::RemoveArgs),

    /// Merge a worktree branch using the configured strategy
    Merge(command::merge::MergeArgs),

    /// Merge, delete branch, and remove worktree (full cleanup)
    Close(command::close::CloseArgs),

    /// Persist session to disk and close panes
    Sleep(command::sleep::SleepArgs),

    /// Rehydrate a sleeping session
    Wake(command::wake::WakeArgs),

    /// Send text to a spirit (e.g. send 1:claude "look at auth")
    Send(command::send::SendArgs),

    /// List all worktrees with spirit and PR status
    List(command::list::ListArgs),

    /// Show active spirits and their states
    Status(command::status::StatusArgs),

    /// Capture a spirit's terminal output
    Capture(command::capture::CaptureArgs),

    /// Block until spirits reach a target status
    Wait(command::wait::WaitArgs),

    /// Focus a spirit's Ghostty window
    Focus(command::focus::FocusArgs),

    /// Checkout a GitHub PR into a worktree
    Checkout(command::checkout::CheckoutArgs),

    /// Scan for git repos in common directories
    Detect(command::detect::DetectArgs),

    /// Interactive project initialization
    Init(command::init::InitArgs),

    /// Remove stale worktrees
    Clean(command::clean::CleanArgs),

    /// Open the TUI dashboard
    Dashboard {
        /// Run the TUI inline in the current terminal (used internally)
        #[arg(long, hide = true)]
        inline: bool,
    },

    /// Manage configuration
    Config(command::config::ConfigArgs),
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        None => {
            // Default: launch TUI dashboard in a Ghostty window
            crate::dashboard::run_in_ghostty().await
        }
        Some(cmd) => match cmd {
            Command::Add(args) => command::add::run(args).await,
            Command::Remove(args) => command::remove::run(args).await,
            Command::Merge(args) => command::merge::run(args).await,
            Command::Close(args) => command::close::run(args).await,
            Command::Sleep(args) => command::sleep::run(args).await,
            Command::Wake(args) => command::wake::run(args).await,
            Command::Send(args) => command::send::run(args).await,
            Command::List(args) => command::list::run(args).await,
            Command::Status(args) => command::status::run(args).await,
            Command::Capture(args) => command::capture::run(args).await,
            Command::Wait(args) => command::wait::run(args).await,
            Command::Focus(args) => command::focus::run(args).await,
            Command::Checkout(args) => command::checkout::run(args).await,
            Command::Detect(args) => command::detect::run(args).await,
            Command::Init(args) => command::init::run(args).await,
            Command::Clean(args) => command::clean::run(args).await,
            Command::Dashboard { inline } => {
                if inline {
                    crate::dashboard::run().await
                } else {
                    crate::dashboard::run_in_ghostty().await
                }
            }
            Command::Config(args) => command::config::run(args).await,
        },
    }
}
