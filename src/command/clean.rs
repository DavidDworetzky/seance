use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct CleanArgs {
    /// Remove worktrees whose remote branch was deleted
    #[arg(long)]
    pub gone: bool,

    /// Remove all closed spirit state files
    #[arg(long)]
    pub closed: bool,
}

pub async fn run(args: CleanArgs) -> Result<()> {
    if args.gone {
        let output = std::process::Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .output()?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut removed = 0;

        for line in stdout.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                if path.contains("__seance") {
                    // Check if the branch's remote tracking is gone
                    let branch_line = stdout
                        .lines()
                        .skip_while(|l| !l.starts_with(&format!("worktree {}", path)))
                        .find(|l| l.starts_with("branch "));

                    if let Some(branch) = branch_line.and_then(|l| l.strip_prefix("branch refs/heads/")) {
                        let remote_check = std::process::Command::new("git")
                            .args(["ls-remote", "--exit-code", "--heads", "origin", branch])
                            .output()?;

                        if !remote_check.status.success() {
                            println!("Removing stale worktree: {} (branch: {})", path, branch);
                            std::process::Command::new("git")
                                .args(["worktree", "remove", "--force", path])
                                .status()?;
                            removed += 1;
                        }
                    }
                }
            }
        }

        println!("Removed {} stale worktrees.", removed);
    }

    if args.closed {
        let store = crate::session::store::SessionStore::load()?;
        let count = store.clean_closed()?;
        println!("Cleaned {} closed state files.", count);
    }

    if !args.gone && !args.closed {
        println!("Specify --gone and/or --closed");
    }

    Ok(())
}
