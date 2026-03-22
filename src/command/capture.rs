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

    let window_title = format!("seance-{}", args.target.replace(':', "-"));
    let output = ghostty.capture_pane(&window_title)?;

    // Take last N lines
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(args.lines);
    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}
