use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    Missing,
}

/// Delete a local branch.
pub fn delete(branch: &str) -> Result<DeleteOutcome> {
    delete_with_flag("-d", branch, "delete")
}

/// Force delete a local branch.
pub fn force_delete(branch: &str) -> Result<DeleteOutcome> {
    delete_with_flag("-D", branch, "force delete")
}

/// Get the current branch name.
pub fn current() -> Result<String> {
    current_in_repo(std::env::current_dir()?.as_path())
}

/// Get the current branch name for a specific repo/worktree.
pub fn current_in_repo(repo_path: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("Failed to get current branch")?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// True for Seance-generated scratch branches that should not be used as add bases.
pub fn is_seance_generated(branch: &str) -> bool {
    if branch.starts_with("seance/") {
        return true;
    }

    let Some(rest) = branch.strip_prefix("seance-") else {
        return false;
    };

    !rest.is_empty()
        && rest
            .split('-')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn delete_with_flag(flag: &str, branch: &str, action: &str) -> Result<DeleteOutcome> {
    let output = std::process::Command::new("git")
        .args(["branch", flag, branch])
        .output()
        .with_context(|| format!("Failed to {}", action))?;

    if output.status.success() {
        return Ok(DeleteOutcome::Deleted);
    }

    let error = command_error(&output);
    if is_missing_branch_error(&error) {
        return Ok(DeleteOutcome::Missing);
    }

    anyhow::bail!("git branch {} failed for {}: {}", flag, branch, error);
}

fn is_missing_branch_error(error: &str) -> bool {
    error.contains("not found")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_seance_generated_matches_old_numeric_branches() {
        assert!(is_seance_generated("seance-1"));
        assert!(is_seance_generated("seance-1-10"));
    }

    #[test]
    fn test_is_seance_generated_matches_namespaced_branches() {
        assert!(is_seance_generated("seance/20260504/example-q1"));
    }

    #[test]
    fn test_is_seance_generated_rejects_user_feature_names() {
        assert!(!is_seance_generated("feature/seance-1"));
        assert!(!is_seance_generated("seance-fix-login"));
        assert!(!is_seance_generated("fix-seance-1"));
    }
}
