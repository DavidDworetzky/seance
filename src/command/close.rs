use anyhow::Result;
use clap::Args;

use crate::config::schema::MergeStrategy;

#[derive(Args)]
pub struct CloseArgs {
    /// Branch to close
    pub branch: String,

    /// Force rebase strategy
    #[arg(long)]
    pub rebase: bool,

    /// Force squash strategy
    #[arg(long)]
    pub squash: bool,

    /// Keep worktree open after merge
    #[arg(long)]
    pub keep: bool,
}

pub async fn run(args: CloseArgs) -> Result<()> {
    let config = crate::config::schema::Config::load(None)?;
    let mut store = crate::session::store::SessionStore::load()?;
    let ghostty = crate::ghostty::GhosttyBackend::new();

    let strategy = if args.rebase {
        MergeStrategy::Rebase
    } else if args.squash {
        MergeStrategy::Squash
    } else {
        config.merge_strategy.clone()
    };

    // 1. Pre-merge hooks
    let wt_path = crate::git::worktree::path_for(&config, &args.branch)?;
    for hook in &config.pre_merge {
        println!("Running pre-merge hook: {}", hook);
        crate::files::ops::run_hook(hook, &wt_path)?;
    }

    // 2. Merge
    crate::git::merge::run(&config, &args.branch, &strategy)?;
    println!("Merged {} using {:?} strategy", args.branch, strategy);

    if !args.keep {
        let quadrant = store.find_quadrant(&args.branch);

        // 3. Remove worktree
        crate::git::worktree::remove(&config, &args.branch)?;

        // 4. Delete branch
        crate::git::branch::delete(&args.branch)?;

        // 5. Close Ghostty window
        let close_result = match quadrant.as_ref() {
            Some(q) => match q.window_id.as_deref() {
                Some(window_id) => ghostty.close_window(window_id),
                None => ghostty.close_window_title(&q.main_window_title()),
            },
            None => Ok(()),
        };
        if let Err(e) = close_result {
            tracing::warn!("Could not close window: {}", e);
        }

        // 6. Remove from session store
        store.remove_quadrant(&args.branch)?;

        println!("Closed: {}", args.branch);
    }

    Ok(())
}
