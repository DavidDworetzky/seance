use anyhow::Result;
use clap::Args;

use crate::config::schema::Config;
use crate::ghostty::GhosttyBackend;
use crate::session::store::{SessionStatus, SessionStore};

#[derive(Args)]
pub struct WakeArgs {
    /// Session name to wake (interactive if omitted)
    pub session: Option<String>,
}

pub async fn run(args: WakeArgs) -> Result<()> {
    let config = Config::load(None)?;
    let ghostty = GhosttyBackend::new();
    let mut store = SessionStore::load()?;

    let session_id = match args.session {
        Some(s) => s,
        None => {
            let sleeping = store.sleeping_sessions();
            if sleeping.is_empty() {
                println!("No sleeping sessions.");
                return Ok(());
            }
            println!("Sleeping sessions:\n");
            for (i, s) in sleeping.iter().enumerate() {
                println!(
                    "  [{}] {}  {} quadrants  sleeping since {}",
                    i + 1,
                    s.name,
                    s.quadrant_count,
                    s.slept_at
                );
            }
            println!();
            // Pick first for now — interactive selection in later phase
            let pick = &sleeping[0];
            println!("Waking: {}", pick.name);
            pick.id.clone()
        }
    };

    // Restore quadrants
    let quadrants = store.quadrants_for(&session_id)?;
    for q in &quadrants {
        // Verify worktree still exists
        if !q.worktree_path.exists() {
            println!(
                "  Warning: worktree {} no longer exists, skipping Q{}",
                q.worktree_path.display(),
                q.quadrant
            );
            continue;
        }

        let bounds = crate::layout::quadrant::compute_bounds(q.quadrant, q.monitor, &config);
        ghostty.create_window(&q.worktree_path, &bounds)?;

        // Re-split for each agent in the group
        let agents: Vec<&String> = q.agents.keys().collect();
        for (i, agent_name) in agents.iter().enumerate() {
            if i > 0 {
                ghostty.split_right()?;
            }
            // Optionally re-launch agent
            if let Some(ac) = config.agents.get(*agent_name) {
                ghostty.send_text(&format!("{}\n", ac.command))?;
            }
            println!("  Restored {} in Q{}", agent_name, q.quadrant);
        }
        ghostty.split_down()?;
    }

    store.set_status(&session_id, SessionStatus::Active)?;
    println!("Session {} restored.", session_id);
    Ok(())
}
