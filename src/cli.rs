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
    /// Enable verbose Ghostty/AppleScript diagnostics
    #[arg(long, global = true)]
    pub debug_ghostty: bool,

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

    /// Select a detected repo and open Seance there
    Repo(command::repo::RepoArgs),

    /// Interactive project initialization
    Init(command::init::InitArgs),

    /// Remove stale worktrees
    Clean(command::clean::CleanArgs),

    /// Open the TUI dashboard
    Dashboard(crate::dashboard::DashboardArgs),

    /// Manage configuration
    Config(command::config::ConfigArgs),
}

pub async fn run(cli: Cli) -> Result<()> {
    let config_debug = crate::config::schema::Config::load(None)
        .map(|config| config.dev.diagnostic_mode)
        .unwrap_or(false);
    crate::debug::set_debug_ghostty(cli.debug_ghostty || config_debug);
    if crate::debug::debug_ghostty() {
        if let Ok(path) = crate::debug::diagnostic_log_path() {
            crate::debug::log(
                "cli",
                &format!(
                    "startup command={:?} diagnostic_log={}",
                    cli.command.as_ref().map(command_name),
                    path.display()
                ),
            );
        }
    }

    match cli.command {
        None => {
            // Default: launch TUI dashboard
            crate::dashboard::run_default().await
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
            Command::Repo(args) => command::repo::run(args).await,
            Command::Init(args) => command::init::run(args).await,
            Command::Clean(args) => command::clean::run(args).await,
            Command::Dashboard(args) => crate::dashboard::run_entry(args).await,
            Command::Config(args) => command::config::run(args).await,
        },
    }
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::Add(_) => "add",
        Command::Remove(_) => "remove",
        Command::Merge(_) => "merge",
        Command::Close(_) => "close",
        Command::Sleep(_) => "sleep",
        Command::Wake(_) => "wake",
        Command::Send(_) => "send",
        Command::List(_) => "list",
        Command::Status(_) => "status",
        Command::Capture(_) => "capture",
        Command::Wait(_) => "wait",
        Command::Focus(_) => "focus",
        Command::Checkout(_) => "checkout",
        Command::Detect(_) => "detect",
        Command::Repo(_) => "repo",
        Command::Init(_) => "init",
        Command::Clean(_) => "clean",
        Command::Dashboard(_) => "dashboard",
        Command::Config(_) => "config",
    }
}
