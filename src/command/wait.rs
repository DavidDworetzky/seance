use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct WaitArgs {
    /// Targets to wait for (quadrant numbers or branch names)
    pub targets: Vec<String>,

    /// Target status to wait for
    #[arg(long, default_value = "done")]
    pub status: String,

    /// Timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Return when ANY target reaches status (default: ALL)
    #[arg(long)]
    pub any: bool,
}

pub async fn run(args: WaitArgs) -> Result<()> {
    let deadline = args
        .timeout
        .map(|t| std::time::Instant::now() + std::time::Duration::from_secs(t));

    loop {
        let store = crate::session::store::SessionStore::load()?;
        let quadrants = store.active_quadrants();

        let mut all_done = true;
        let mut any_done = false;

        for target in &args.targets {
            let matched = quadrants.iter().any(|q| {
                let matches = q.branch == *target || q.quadrant.to_string() == *target;
                if !matches {
                    return false;
                }
                q.agents
                    .values()
                    .all(|a| format!("{:?}", a.status).to_lowercase() == args.status)
            });

            if matched {
                any_done = true;
            } else {
                all_done = false;
            }
        }

        if (args.any && any_done) || all_done {
            return Ok(());
        }

        if let Some(d) = deadline {
            if std::time::Instant::now() >= d {
                std::process::exit(124); // timeout exit code
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
