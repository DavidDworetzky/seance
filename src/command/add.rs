use anyhow::Result;
use clap::Args;

use crate::agent;
use crate::config::schema::Config;
use crate::ghostty::GhosttyBackend;
use crate::git::worktree;
use crate::layout::quadrant::QuadrantAssigner;
use crate::session::store::{self, SessionStore};

#[derive(Args)]
pub struct AddArgs {
    /// Branch name (auto-generated if omitted)
    #[arg(short, long)]
    pub branch: Option<String>,

    /// Inline prompt for the spirits
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Prompt file
    #[arg(short = 'P', long)]
    pub prompt_file: Option<String>,

    /// Override agent (replaces entire group for this quadrant)
    #[arg(long)]
    pub agent: Option<String>,

    /// Target quadrant (1-8)
    #[arg(short, long)]
    pub quadrant: Option<u8>,

    /// Target monitor (0 = primary)
    #[arg(short, long, default_value = "0")]
    pub monitor: u8,

    /// Base branch (default: main)
    #[arg(long)]
    pub base: Option<String>,

    /// LLM-generated branch name from prompt
    #[arg(short = 'A', long)]
    pub auto_name: bool,

    /// Skip file copy/symlink operations
    #[arg(long)]
    pub no_file_ops: bool,

    /// Create all quadrants on a monitor at once
    #[arg(long)]
    pub circle: bool,
}

pub async fn run(args: AddArgs) -> Result<()> {
    let config = Config::load(None)?;
    let mut store = SessionStore::load()?;
    let ghostty = GhosttyBackend::new();
    let assigner = QuadrantAssigner::new(&store, config.quadrants_per_monitor);

    let repo_path = std::env::current_dir()?.to_string_lossy().to_string();
    let session_name = args.branch.as_deref().unwrap_or("seance");
    let session_id = store.ensure_active_session(session_name, &repo_path)?;

    if args.circle {
        let count = config.quadrants_per_monitor;
        println!("Creating circle with {} worktrees on monitor {}...", count, args.monitor);

        for q in 1..=count {
            let branch = match &args.branch {
                Some(b) => format!("{}-{}", b, q),
                None => format!("seance-{}", q),
            };
            create_quadrant(&config, &ghostty, &mut store, &session_id, &branch, q, args.monitor, &args)?;
        }
    } else {
        let branch = args.branch.clone().unwrap_or_else(|| "seance-1".into());
        let quadrant = args.quadrant.unwrap_or_else(|| assigner.next_available(args.monitor));
        create_quadrant(&config, &ghostty, &mut store, &session_id, &branch, quadrant, args.monitor, &args)?;
    }

    Ok(())
}

fn create_quadrant(
    config: &Config,
    ghostty: &GhosttyBackend,
    store: &mut SessionStore,
    session_id: &str,
    branch: &str,
    quadrant: u8,
    monitor: u8,
    args: &AddArgs,
) -> Result<()> {
    let base = args
        .base
        .as_deref()
        .or(config.base_branch.as_deref())
        .unwrap_or(&config.main_branch);

    // 1. Create worktree
    let wt_path = worktree::create(config, branch, base)?;
    println!("  Created worktree: {}", wt_path.display());

    // 2. File operations
    if !args.no_file_ops {
        crate::files::ops::run_file_ops(config, &wt_path)?;
    }

    // 3. Post-create hooks
    for hook in &config.post_create {
        crate::files::ops::run_hook(hook, &wt_path)?;
    }

    // 4. Determine agents for this quadrant
    let agents: Vec<String> = if let Some(ref a) = args.agent {
        vec![a.clone()]
    } else {
        config.group.clone()
    };

    // 5. Resolve prompt if provided
    let prompt_file = if let Some(ref text) = args.prompt {
        Some(crate::agent::prompt::write_prompt_file(&wt_path, branch, text)?)
    } else if let Some(ref path) = args.prompt_file {
        let content = std::fs::read_to_string(path)?;
        Some(crate::agent::prompt::write_prompt_file(&wt_path, branch, &content)?)
    } else {
        None
    };

    // 6. Open Ghostty window in quadrant
    let bounds = crate::layout::quadrant::compute_bounds(quadrant, monitor, config);
    ghostty.create_window(&wt_path, &bounds)?;

    // 7. Split panes for each agent + launch
    for (i, agent_name) in agents.iter().enumerate() {
        if i > 0 {
            ghostty.split_right()?;
        }
        if let Some(ac) = config.agents.get(agent_name) {
            let cmd = agent::build_launch_command(ac, prompt_file.as_deref());
            ghostty.send_text(&format!("{}\n", cmd))?;
        }
        println!("  Started {} in Q{}", agent_name, quadrant);
    }

    // 8. Shell pane at the bottom
    ghostty.split_down()?;

    // 9. Register in session store
    let quadrant_state = store::new_quadrant_state(quadrant, monitor, branch, wt_path, &agents);
    store.add_quadrant(session_id, quadrant_state)?;

    println!("  Q{} ready on monitor {}", quadrant, monitor);
    Ok(())
}
