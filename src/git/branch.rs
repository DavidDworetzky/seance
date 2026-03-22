use anyhow::{Context, Result};

/// Delete a local branch.
pub fn delete(branch: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["branch", "-d", branch])
        .status()
        .context("Failed to delete branch")?;

    if !status.success() {
        anyhow::bail!("git branch -d failed for: {}", branch);
    }

    Ok(())
}

/// Force delete a local branch.
pub fn force_delete(branch: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["branch", "-D", branch])
        .status()
        .context("Failed to force delete branch")?;

    if !status.success() {
        anyhow::bail!("git branch -D failed for: {}", branch);
    }

    Ok(())
}

/// Get the current branch name.
pub fn current() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to get current branch")?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
