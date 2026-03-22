use anyhow::Result;
use clap::Args;

use crate::ghostty::GhosttyBackend;
use crate::session::store::{SessionStatus, SessionStore};

#[derive(Args)]
pub struct SleepArgs {
    /// Session name (defaults to current)
    pub session: Option<String>,

    /// Keep Ghostty windows open
    #[arg(long)]
    pub keep_windows: bool,
}

pub async fn run(args: SleepArgs) -> Result<()> {
    let ghostty = GhosttyBackend::new();
    let mut store = SessionStore::load()?;

    let session_id = args.session.unwrap_or_else(|| store.current_session_id());

    let quadrants = store.quadrants_for(&session_id)?;
    if quadrants.is_empty() {
        println!("No active quadrants in session {}.", session_id);
        return Ok(());
    }

    // Capture terminal state from each quadrant
    for q in &quadrants {
        for (agent_name, _) in &q.agents {
            match ghostty.capture_pane(&q.window_title(agent_name)) {
                Ok(snapshot) => {
                    store.save_snapshot(&session_id, q.quadrant, agent_name, &snapshot)?;
                    println!("  Captured Q{}:{}", q.quadrant, agent_name);
                }
                Err(e) => {
                    tracing::warn!("Could not capture Q{}:{}: {}", q.quadrant, agent_name, e);
                }
            }
        }
    }

    // Mark session as sleeping
    store.set_status(&session_id, SessionStatus::Sleeping)?;

    // Close windows unless --keep-windows
    if !args.keep_windows {
        for q in &quadrants {
            if let Err(e) = ghostty.close_window(&q.main_window_title()) {
                tracing::warn!("Could not close window for Q{}: {}", q.quadrant, e);
            }
        }
    }

    println!("Session {} is now sleeping.", session_id);
    Ok(())
}
