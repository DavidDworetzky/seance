use anyhow::{Context, Result};

use crate::config::schema::MergeStrategy;

/// Execute a merge using the specified strategy.
pub fn run(
    config: &crate::config::schema::Config,
    branch: &str,
    strategy: &MergeStrategy,
) -> Result<()> {
    let main = config.base_branch.as_deref().unwrap_or(&config.main_branch);

    // Checkout main branch first
    let status = std::process::Command::new("git")
        .args(["checkout", main])
        .status()
        .context("Failed to checkout main branch")?;

    if !status.success() {
        anyhow::bail!("git checkout {} failed", main);
    }

    match strategy {
        MergeStrategy::Merge => {
            let status = std::process::Command::new("git")
                .args(["merge", branch])
                .status()
                .context("git merge failed")?;
            if !status.success() {
                anyhow::bail!("git merge {} failed", branch);
            }
        }
        MergeStrategy::Rebase => {
            // Rebase the feature branch onto main, then fast-forward
            let status = std::process::Command::new("git")
                .args(["rebase", main, branch])
                .status()
                .context("git rebase failed")?;
            if !status.success() {
                anyhow::bail!("git rebase {} {} failed", main, branch);
            }

            let status = std::process::Command::new("git")
                .args(["checkout", main])
                .status()?;
            if !status.success() {
                anyhow::bail!("git checkout {} failed after rebase", main);
            }

            let status = std::process::Command::new("git")
                .args(["merge", "--ff-only", branch])
                .status()?;
            if !status.success() {
                anyhow::bail!("git merge --ff-only {} failed", branch);
            }
        }
        MergeStrategy::Squash => {
            let status = std::process::Command::new("git")
                .args(["merge", "--squash", branch])
                .status()
                .context("git merge --squash failed")?;
            if !status.success() {
                anyhow::bail!("git merge --squash {} failed", branch);
            }

            let status = std::process::Command::new("git")
                .args(["commit", "-m", &format!("squash merge: {}", branch)])
                .status()?;
            if !status.success() {
                anyhow::bail!("git commit failed after squash merge");
            }
        }
    }

    Ok(())
}
