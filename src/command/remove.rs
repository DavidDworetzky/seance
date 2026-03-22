use anyhow::Result;
use clap::Args;

use crate::config::schema::Config;
use crate::ghostty::GhosttyBackend;
use crate::session::store::SessionStore;

#[derive(Args)]
pub struct RemoveArgs {
    /// Branch name or quadrant number to remove
    pub target: Option<String>,

    /// Remove entire circle
    #[arg(long)]
    pub circle: bool,
}

pub async fn run(args: RemoveArgs) -> Result<()> {
    let config = Config::load(None)?;
    let mut store = SessionStore::load()?;
    let ghostty = GhosttyBackend::new();

    if args.circle {
        println!("Removing all worktrees in circle...");
        let branches: Vec<String> = store.active_quadrants().iter().map(|q| q.branch.clone()).collect();
        for branch in branches {
            remove_one(&config, &ghostty, &mut store, &branch)?;
        }
    } else if let Some(target) = &args.target {
        // Resolve target — could be quadrant number or branch name
        let branch = resolve_target(&store, target)?;
        remove_one(&config, &ghostty, &mut store, &branch)?;
    } else {
        anyhow::bail!("Specify a branch/quadrant to remove, or use --circle");
    }

    Ok(())
}

fn resolve_target(store: &SessionStore, target: &str) -> Result<String> {
    // Try as quadrant number
    if let Ok(num) = target.parse::<u8>() {
        if let Some(q) = store.active_quadrants().iter().find(|q| q.quadrant == num) {
            return Ok(q.branch.clone());
        }
        anyhow::bail!("No active worktree in quadrant {}", num);
    }
    Ok(target.to_string())
}

fn remove_one(
    config: &Config,
    ghostty: &GhosttyBackend,
    store: &mut SessionStore,
    branch: &str,
) -> Result<()> {
    // Close Ghostty window
    if let Err(e) = ghostty.close_window(branch) {
        tracing::warn!("Could not close window for {}: {}", branch, e);
    }

    // Remove worktree
    if let Err(e) = crate::git::worktree::remove(config, branch) {
        tracing::warn!("Could not remove worktree for {}: {}", branch, e);
    }

    // Delete branch
    if let Err(e) = crate::git::branch::delete(branch) {
        tracing::warn!("Could not delete branch {}: {}", branch, e);
    }

    // Remove from session store
    store.remove_quadrant(branch)?;

    println!("Removed: {}", branch);
    Ok(())
}
