use anyhow::Result;
use clap::Args;

use crate::ghostty::GhosttyBackend;
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

    let quadrant = if args.next {
        store.next_quadrant()
    } else if args.prev {
        store.prev_quadrant()
    } else {
        args.quadrant.unwrap_or(1)
    };

    let q = store
        .active_quadrants()
        .into_iter()
        .find(|q| q.quadrant == quadrant)
        .ok_or_else(|| anyhow::anyhow!("No active worktree in quadrant {}", quadrant))?;

    ghostty.focus_window(&q.main_window_title())?;
    println!("Focused Q{} ({})", quadrant, q.branch);

    Ok(())
}
