use anyhow::Result;
use clap::Args;

use crate::config::schema::MergeStrategy;

#[derive(Args)]
pub struct MergeArgs {
    /// Branch to merge
    pub branch: String,

    /// Force rebase strategy
    #[arg(long)]
    pub rebase: bool,

    /// Force squash strategy
    #[arg(long)]
    pub squash: bool,
}

pub async fn run(args: MergeArgs) -> Result<()> {
    let config = crate::config::schema::Config::load(None)?;

    let strategy = if args.rebase {
        MergeStrategy::Rebase
    } else if args.squash {
        MergeStrategy::Squash
    } else {
        config.merge_strategy.clone()
    };

    // Run pre-merge hooks
    for hook in &config.pre_merge {
        println!("Running pre-merge hook: {}", hook);
        let wt_path = crate::git::worktree::path_for(&config, &args.branch)?;
        crate::files::ops::run_hook(hook, &wt_path)?;
    }

    crate::git::merge::run(&config, &args.branch, &strategy)?;
    println!("Merged {} using {:?} strategy", args.branch, strategy);

    Ok(())
}
