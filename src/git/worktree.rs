use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::schema::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOutcome {
    Removed,
    Missing,
}

/// Resolve the worktree directory for a given config.
fn worktree_base(config: &Config) -> Result<PathBuf> {
    let repo_path = std::env::current_dir()?;
    worktree_base_for(config, &repo_path)
}

fn worktree_base_for(config: &Config, repo_path: &Path) -> Result<PathBuf> {
    let project_name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());

    let dir = config.worktree_dir.replace("{project}", &project_name);

    let base = if dir.starts_with('/') {
        PathBuf::from(&dir)
    } else {
        repo_path.join(&dir)
    };

    Ok(base)
}

/// Create a new git worktree for the given branch.
pub fn create(config: &Config, branch: &str, base_branch: &str) -> Result<PathBuf> {
    let repo_path = std::env::current_dir()?;
    create_in_repo(config, &repo_path, branch, base_branch)
}

/// Create a new git worktree for the given branch in a specific repo.
pub fn create_in_repo(
    config: &Config,
    repo_path: &Path,
    branch: &str,
    base_branch: &str,
) -> Result<PathBuf> {
    let wt_base = worktree_base_for(config, repo_path)?;
    let sanitized = branch.replace('/', "-");
    let wt_path = wt_base.join(&sanitized);

    // Ensure parent directory exists
    if let Some(parent) = wt_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            &wt_path.to_string_lossy(),
            "-b",
            branch,
            base_branch,
        ])
        .current_dir(repo_path)
        .output()
        .context("Failed to create git worktree")?;

    if !output.status.success() {
        anyhow::bail!(
            "git worktree add failed for {}: {}",
            branch,
            command_error(&output)
        );
    }

    Ok(wt_path)
}

/// Remove a git worktree by branch name.
pub fn remove(config: &Config, branch: &str) -> Result<RemoveOutcome> {
    let wt_path = path_for(config, branch)?;
    remove_path(&wt_path, branch)
}

/// Remove a git worktree by its path.
pub fn remove_path(wt_path: &Path, branch: &str) -> Result<RemoveOutcome> {
    if !looks_like_worktree(wt_path) {
        prune_stale()?;
        return Ok(RemoveOutcome::Missing);
    }

    let output = std::process::Command::new("git")
        .args(["worktree", "remove", "--force", &wt_path.to_string_lossy()])
        .output()
        .context("Failed to remove git worktree")?;

    if output.status.success() {
        return Ok(RemoveOutcome::Removed);
    }

    let error = command_error(&output);
    if is_missing_worktree_error(&error) {
        prune_stale()?;
        return Ok(RemoveOutcome::Missing);
    }

    anyhow::bail!("git worktree remove failed for {}: {}", branch, error);
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
            let name = entry.file_name().to_string_lossy().to_string();
            result.push((name, path));
        }
    }

    Ok(result)
}

fn looks_like_worktree(path: &Path) -> bool {
    path.join(".git").exists()
}

fn prune_stale() -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["worktree", "prune", "--expire", "now"])
        .output()
        .context("Failed to prune stale git worktrees")?;

    if output.status.success() {
        return Ok(());
    }

    anyhow::bail!("git worktree prune failed: {}", command_error(&output));
}

fn is_missing_worktree_error(error: &str) -> bool {
    error.contains("is not a working tree")
}

fn command_error(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }

    "git command failed".to_string()
}
