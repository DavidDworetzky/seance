use anyhow::{Context, Result};
use clap::Args;
use std::path::Path;

use crate::agent;
use crate::config::schema::Config;
use crate::ghostty::{GhosttyBackend, TerminalInput};
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
    let repo_path = crate::git::repo_root(&std::env::current_dir()?)?;
    run_in_repo(args, &repo_path)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct CreatedQuadrant {
    pub branch: String,
    pub quadrant: u8,
    pub monitor: u8,
}

pub fn run_in_repo(args: AddArgs, repo_path: &Path) -> Result<Vec<CreatedQuadrant>> {
    let config = Config::load(Some(repo_path))?;
    let mut store = SessionStore::load()?;
    let ghostty = GhosttyBackend::new();
    let assigner = QuadrantAssigner::new(&store, config.quadrants_per_monitor);
    let prompt_content =
        crate::agent::prompt::resolve_prompt(args.prompt.as_deref(), args.prompt_file.as_deref())?;

    let repo_path = crate::git::repo_root(repo_path)?
        .to_string_lossy()
        .to_string();
    let auto_name_requested = args.auto_name || config.auto_name.enabled;
    let branch_seed = match args.branch.clone() {
        Some(branch) => Some(branch),
        None if auto_name_requested => {
            let prompt = prompt_content
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("--auto-name requires --prompt or --prompt-file"))?;
            Some(agent::generate_branch_name(&config, prompt)?)
        }
        None => None,
    };

    let session_name = branch_seed.as_deref().unwrap_or("seance");
    let session_id = store.ensure_active_session(session_name, &repo_path)?;
    let mut created = Vec::new();
    let repo_path = Path::new(&repo_path);
    crate::debug::log(
        "add",
        &format!(
            "run_in_repo repo={} session_id={} circle={} monitor={}",
            repo_path.display(),
            session_id,
            args.circle,
            args.monitor
        ),
    );

    if args.circle {
        let count = config.quadrants_per_monitor;
        println!(
            "Creating circle with {} worktrees on monitor {}...",
            count, args.monitor
        );

        for q in 1..=count {
            let branch = branch_seed
                .as_deref()
                .map(|seed| format!("{}-{}", seed, q))
                .unwrap_or_else(|| unique_default_branch(repo_path, q));
            created.push(create_quadrant(
                &config,
                &ghostty,
                &mut store,
                &session_id,
                repo_path,
                &branch,
                q,
                args.monitor,
                &args,
                prompt_content.as_deref(),
            )?);
        }
    } else {
        let quadrant = args
            .quadrant
            .unwrap_or_else(|| assigner.next_available_for(&store, args.monitor));
        let branch = branch_seed.unwrap_or_else(|| unique_default_branch(repo_path, quadrant));
        created.push(create_quadrant(
            &config,
            &ghostty,
            &mut store,
            &session_id,
            repo_path,
            &branch,
            quadrant,
            args.monitor,
            &args,
            prompt_content.as_deref(),
        )?);
    }

    Ok(created)
}

fn unique_default_branch(repo_path: &Path, quadrant: u8) -> String {
    let base = format!("seance-{}", quadrant);
    if !branch_exists(repo_path, &base) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{}-{}", base, suffix);
        if !branch_exists(repo_path, &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn branch_exists(repo_path: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{}", branch),
        ])
        .current_dir(repo_path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn create_quadrant(
    config: &Config,
    ghostty: &GhosttyBackend,
    store: &mut SessionStore,
    session_id: &str,
    repo_path: &Path,
    branch: &str,
    quadrant: u8,
    monitor: u8,
    args: &AddArgs,
    prompt_content: Option<&str>,
) -> Result<CreatedQuadrant> {
    let base = args
        .base
        .as_deref()
        .or(config.base_branch.as_deref())
        .unwrap_or(&config.main_branch);
    let target = format!(
        "repo={} branch={} quadrant={} monitor={}",
        repo_path.display(),
        branch,
        quadrant,
        monitor
    );
    crate::debug::log("add", &format!("start {}", target));

    // 1. Create worktree
    let wt_path = worktree::create_in_repo(config, repo_path, branch, base)
        .with_context(|| format!("add flow step=create_worktree {}", target))?;
    crate::debug::log(
        "add",
        &format!("created worktree {} {}", wt_path.display(), target),
    );
    println!("  Created worktree: {}", wt_path.display());

    // 2. File operations
    if !args.no_file_ops {
        crate::files::ops::run_file_ops(config, &wt_path)
            .with_context(|| format!("add flow step=file_ops {}", target))?;
        crate::debug::log("add", &format!("file ops complete {}", target));
    }

    // 3. Post-create hooks
    for hook in &config.post_create {
        crate::files::ops::run_hook(hook, &wt_path)
            .with_context(|| format!("add flow step=post_create hook={} {}", hook, target))?;
        crate::debug::log("add", &format!("post-create hook={} {}", hook, target));
    }

    // 4. Determine agents for this quadrant
    let agents: Vec<String> = if let Some(ref a) = args.agent {
        vec![a.clone()]
    } else {
        config.group.clone()
    };

    // 5. Resolve prompt if provided
    let prompt_file = prompt_content
        .map(|content| crate::agent::prompt::write_prompt_file(&wt_path, branch, content))
        .transpose()
        .with_context(|| format!("add flow step=write_prompt {}", target))?;
    crate::debug::log(
        "add",
        &format!(
            "prompt prepared prompt_file={:?} {}",
            prompt_file.as_deref(),
            target
        ),
    );

    // 6. Open Ghostty window in quadrant
    let bounds = crate::layout::quadrant::compute_bounds(quadrant, monitor, config);
    crate::debug::log(
        "add",
        &format!(
            "create_window bounds={{x:{}, y:{}, width:{}, height:{}}} {}",
            bounds.x, bounds.y, bounds.width, bounds.height, target
        ),
    );
    let first_input = agents
        .first()
        .and_then(|agent_name| config.agents.get(agent_name))
        .map(|ac| {
            TerminalInput::new(format!(
                "{}\n",
                agent::build_launch_command(ac, prompt_file.as_deref())
            ))
        });

    let window = ghostty
        .create_window_with_input(&wt_path, &bounds, first_input.as_ref())
        .with_context(|| format!("add flow step=create_window {}", target))?;
    let window_id = window.window_id.clone();
    crate::debug::log(
        "add",
        &format!(
            "window ready window_id={} terminal_id={} {}",
            window.window_id, window.terminal_id, target
        ),
    );

    // 7. Split panes for each agent + launch
    let mut current_terminal = window.terminal_id.clone();
    let mut pane_ids = Vec::with_capacity(agents.len());
    for (i, agent_name) in agents.iter().enumerate() {
        let launch_input = config.agents.get(agent_name).map(|ac| {
            TerminalInput::new(format!(
                "{}\n",
                agent::build_launch_command(ac, prompt_file.as_deref())
            ))
        });

        if i > 0 {
            current_terminal = ghostty
                .split_right_with_input(&window_id, launch_input.as_ref())
                .with_context(|| {
                    format!("add flow step=split_right agent={} {}", agent_name, target)
                })?;
            crate::debug::log(
                "add",
                &format!(
                    "split_right agent={} terminal={} {}",
                    agent_name, current_terminal, target
                ),
            );
        }
        if let Some(ac) = config.agents.get(agent_name) {
            let cmd = agent::build_launch_command(ac, prompt_file.as_deref());
            crate::debug::log(
                "add",
                &format!(
                    "launch_agent agent={} terminal={} command={:?} {}",
                    agent_name, current_terminal, cmd, target
                ),
            );
        }
        pane_ids.push((agent_name.clone(), current_terminal.clone()));
        println!("  Started {} in Q{}", agent_name, quadrant);
    }

    // 8. Shell pane at the bottom
    if let Some((_, terminal_id)) = pane_ids.last() {
        let _ = ghostty.split_down(&window_id).with_context(|| {
            format!(
                "add flow step=split_down terminal={} {}",
                terminal_id, target
            )
        })?;
        crate::debug::log(
            "add",
            &format!("split_down terminal={} {}", terminal_id, target),
        );
    }

    // 9. Register in session store
    let mut quadrant_state = store::new_quadrant_state(quadrant, monitor, branch, wt_path, &agents);
    quadrant_state.window_id = Some(window.window_id.to_string());
    for (agent_name, pane_id) in pane_ids {
        if let Some(spirit) = quadrant_state.agents.get_mut(&agent_name) {
            spirit.pane_id = Some(pane_id.to_string());
        }
    }
    store
        .add_quadrant(session_id, quadrant_state)
        .with_context(|| format!("add flow step=store_quadrant {}", target))?;
    crate::debug::log("add", &format!("stored quadrant {}", target));

    println!("  Q{} ready on monitor {}", quadrant, monitor);
    Ok(CreatedQuadrant {
        branch: branch.to_string(),
        quadrant,
        monitor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_git_repo(path: &Path) {
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());

        let status = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=Test User",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_unique_default_branch_uses_base_name_when_available() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());
        assert_eq!(unique_default_branch(dir.path(), 1), "seance-1");
    }

    #[test]
    fn test_unique_default_branch_increments_when_branch_exists() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        let status = std::process::Command::new("git")
            .args(["branch", "seance-1"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        assert!(status.success());

        assert_eq!(unique_default_branch(dir.path(), 1), "seance-1-2");
    }
}
