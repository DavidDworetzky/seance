use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::config::schema::Config;

/// Resolve the worktree directory for a given config.
fn worktree_base(config: &Config) -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let project_name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());

    let dir = config
        .worktree_dir
        .replace("{project}", &project_name);

    let base = if dir.starts_with('/') {
        PathBuf::from(&dir)
    } else {
        cwd.join(&dir)
    };

    Ok(base)
}

/// Create a new git worktree for the given branch.
pub fn create(config: &Config, branch: &str, base_branch: &str) -> Result<PathBuf> {
    let wt_base = worktree_base(config)?;
    let sanitized = branch.replace('/', "-");
    let wt_path = wt_base.join(&sanitized);

    // Ensure parent directory exists
    if let Some(parent) = wt_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let status = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            &wt_path.to_string_lossy(),
            "-b",
            branch,
            base_branch,
        ])
        .status()
        .context("Failed to create git worktree")?;

    if !status.success() {
        anyhow::bail!("git worktree add failed for branch: {}", branch);
    }

    Ok(wt_path)
}

/// Remove a git worktree by branch name.
pub fn remove(config: &Config, branch: &str) -> Result<()> {
    let wt_path = path_for(config, branch)?;

    let status = std::process::Command::new("git")
        .args(["worktree", "remove", "--force", &wt_path.to_string_lossy()])
        .status()
        .context("Failed to remove git worktree")?;

    if !status.success() {
        anyhow::bail!("git worktree remove failed for: {}", branch);
    }

    Ok(())
}

/// Get the worktree path for a branch.
pub fn path_for(config: &Config, branch: &str) -> Result<PathBuf> {
    let wt_base = worktree_base(config)?;
    let sanitized = branch.replace('/', "-");
    Ok(wt_base.join(&sanitized))
}

/// List all seance-managed worktrees.
pub fn list(config: &Config) -> Result<Vec<(String, PathBuf)>> {
    let wt_base = worktree_base(config)?;
    let mut result = Vec::new();

    if !wt_base.exists() {
        return Ok(result);
    }

    for entry in std::fs::read_dir(&wt_base)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(".git").exists() {
            let name = entry
                .file_name()
                .to_string_lossy()
                .to_string();
            result.push((name, path));
        }
    }

    Ok(result)
}
