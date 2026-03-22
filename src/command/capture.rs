use crate::session::store::SessionStore;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct CaptureArgs {
    /// Target in format "quadrant:agent" (e.g. "1:claude")
    pub target: String,

    /// Number of lines to capture
    #[arg(short, long, default_value = "100")]
    pub lines: usize,
}

pub async fn run(args: CaptureArgs) -> Result<()> {
    let ghostty = crate::ghostty::GhosttyBackend::new();
    let store = SessionStore::load()?;
    let (quadrant_str, agent_name) = args
        .target
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Target must be in format quadrant:agent"))?;
    let quadrant = store
        .find_quadrant(quadrant_str)
        .ok_or_else(|| anyhow::anyhow!("No active worktree in quadrant {}", quadrant_str))?;
    let output = if let Some(pane_id) = quadrant.pane_id(agent_name) {
        ghostty.capture_pane(pane_id)?
    } else {
        ghostty.capture_pane_title(&quadrant.window_title(agent_name))?
    };

    // Take last N lines
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(args.lines);
    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}
