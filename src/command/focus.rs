use anyhow::Result;
use clap::Args;

use crate::ghostty::{GhosttyBackend, WindowId, WindowTitle};
use crate::session::store::SessionStore;

#[derive(Args)]
pub struct FocusArgs {
    /// Quadrant number to focus
    pub quadrant: Option<u8>,

    /// Focus next quadrant
    #[arg(long)]
    pub next: bool,

    /// Focus previous quadrant
    #[arg(long)]
    pub prev: bool,
}

pub async fn run(args: FocusArgs) -> Result<()> {
    let ghostty = GhosttyBackend::new();
    let store = SessionStore::load()?;
    let mut quadrants = store.active_quadrants();
    quadrants.sort_by_key(|q| (q.monitor, q.quadrant));

    let q = if args.next || args.prev {
        let front_window = ghostty.front_window_id().ok();
        let current_index = front_window.as_ref().and_then(|front| {
            quadrants
                .iter()
                .position(|q| q.window_id.as_deref() == Some(front.as_ref()))
        });
        let index = if args.next {
            match current_index {
                Some(i) => (i + 1) % quadrants.len().max(1),
                None => 0,
            }
        } else {
            match current_index {
                Some(0) | None => quadrants.len().saturating_sub(1),
                Some(i) => i.saturating_sub(1),
            }
        };
        quadrants
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No active worktrees"))?
    } else {
        let quadrant = args.quadrant.unwrap_or(1);
        quadrants
            .into_iter()
            .find(|q| q.quadrant == quadrant)
            .ok_or_else(|| anyhow::anyhow!("No active worktree in quadrant {}", quadrant))?
    };

    match q.window_id.as_deref() {
        Some(window_id) => {
            let window_id = WindowId::new(window_id.to_string())?;
            ghostty.focus_window(&window_id)?
        }
        None => {
            let window_title = WindowTitle::new(q.main_window_title())?;
            ghostty.focus_window_title(&window_title)?
        }
    }
    println!("Focused Q{} ({})", q.quadrant, q.branch);

    Ok(())
}
