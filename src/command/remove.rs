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
        let quadrants = store.active_quadrants();
        for quadrant in quadrants {
            remove_one(&config, &ghostty, &mut store, &quadrant)?;
        }
    } else if let Some(target) = &args.target {
        let quadrant = store
            .find_quadrant(target)
            .ok_or_else(|| anyhow::anyhow!("No active worktree found for {}", target))?;
        remove_one(&config, &ghostty, &mut store, &quadrant)?;
    } else {
        anyhow::bail!("Specify a branch/quadrant to remove, or use --circle");
    }

    Ok(())
}

fn remove_one(
    config: &Config,
    ghostty: &GhosttyBackend,
    store: &mut SessionStore,
    quadrant: &crate::session::store::QuadrantState,
) -> Result<()> {
    // Close Ghostty window
    let close_result = match quadrant.window_id.as_deref() {
        Some(window_id) => ghostty.close_window(window_id),
        None => ghostty.close_window_title(&quadrant.main_window_title()),
    };
    if let Err(e) = close_result {
        tracing::warn!("Could not close window for {}: {}", quadrant.branch, e);
    }

    // Remove worktree
    if let Err(e) = crate::git::worktree::remove(config, &quadrant.branch) {
        tracing::warn!("Could not remove worktree for {}: {}", quadrant.branch, e);
    }

    // Delete branch
    if let Err(e) = crate::git::branch::delete(&quadrant.branch) {
        tracing::warn!("Could not delete branch {}: {}", quadrant.branch, e);
    }

    // Remove from session store
    store.remove_quadrant(&quadrant.branch)?;

    println!("Removed: {}", quadrant.branch);
    Ok(())
}
