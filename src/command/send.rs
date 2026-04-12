use anyhow::Result;
use clap::Args;

use crate::ghostty::{GhosttyBackend, TerminalId, TerminalInput, WindowTitle};
use crate::session::store::SessionStore;

#[derive(Args)]
pub struct SendArgs {
    /// Target in format "quadrant:agent" (e.g. "1:claude") or just "1" for all agents
    pub target: String,

    /// Text to send
    pub text: Option<String>,

    /// Read text from file
    #[arg(short, long)]
    pub file: Option<String>,
}

pub async fn run(args: SendArgs) -> Result<()> {
    let config = crate::config::schema::Config::load(None)?;
    let store = SessionStore::load()?;
    let ghostty = GhosttyBackend::new();

    let text = if let Some(file) = &args.file {
        std::fs::read_to_string(file)?
    } else if let Some(text) = &args.text {
        text.clone()
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    };

    // Parse target: "1:claude" or just "1"
    let (quadrant_str, agent_filter) = if args.target.contains(':') {
        let parts: Vec<&str> = args.target.splitn(2, ':').collect();
        (parts[0], Some(parts[1].to_string()))
    } else {
        (args.target.as_str(), None)
    };

    let quadrant_num: u8 = quadrant_str.parse()?;

    let quadrant = store
        .active_quadrants()
        .into_iter()
        .find(|q| q.quadrant == quadrant_num)
        .ok_or_else(|| anyhow::anyhow!("No active worktree in quadrant {}", quadrant_num))?;

    let agents_to_send: Vec<String> = match agent_filter {
        Some(name) => vec![name],
        None => quadrant.ordered_agent_names(&config.group),
    };

    for agent_name in &agents_to_send {
        if let Some(pane_id) = quadrant.pane_id(agent_name) {
            let pane_id = TerminalId::new(pane_id.to_string())?;
            let text = TerminalInput::new(text.clone());
            ghostty.send_text(&pane_id, &text)?;
        } else {
            let window_title = WindowTitle::new(quadrant.window_title(agent_name))?;
            let text = TerminalInput::new(text.clone());
            ghostty.send_text_to_window(&window_title, &text)?;
        }
        println!("Sent to Q{}:{}", quadrant_num, agent_name);
    }

    Ok(())
}
