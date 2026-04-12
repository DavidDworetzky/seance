use anyhow::{Context, Result};

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
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to get current branch")?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
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
