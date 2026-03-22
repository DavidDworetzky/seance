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

    let session_id = match args.session {
        Some(session_id) => session_id,
        None => store
            .current_session_id()
            .ok_or_else(|| anyhow::anyhow!("No active session to sleep"))?,
    };

    let quadrants = store.quadrants_for(&session_id)?;
    if quadrants.is_empty() {
        println!("No active quadrants in session {}.", session_id);
        return Ok(());
    }

    // Capture terminal state from each quadrant
    for q in &quadrants {
        let mut agent_names: Vec<String> = q.agents.keys().cloned().collect();
        agent_names.sort();
        for agent_name in agent_names {
            let capture = if let Some(pane_id) = q.pane_id(&agent_name) {
                ghostty.capture_pane(pane_id)
            } else {
                ghostty.capture_pane_title(&q.window_title(&agent_name))
            };

            match capture {
                Ok(snapshot) => {
                    store.save_snapshot(&session_id, q.quadrant, &agent_name, &snapshot)?;
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
            let close_result = match q.window_id.as_deref() {
                Some(window_id) => ghostty.close_window(window_id),
                None => ghostty.close_window_title(&q.main_window_title()),
            };

            if let Err(e) = close_result {
                tracing::warn!("Could not close window for Q{}: {}", q.quadrant, e);
            }
        }
    }

    println!("Session {} is now sleeping.", session_id);
    Ok(())
}
