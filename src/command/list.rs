use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct ListArgs {
    /// Show PR status (requires gh CLI)
    #[arg(long)]
    pub pr: bool,
}

pub async fn run(args: ListArgs) -> Result<()> {
    let config = crate::config::schema::Config::load(None)?;
    let mut store = crate::session::store::SessionStore::load()?;

    // Optionally fetch PR status
    if args.pr {
        fetch_pr_status(&mut store)?;
    }

    let quadrants = store.active_quadrants();
    if quadrants.is_empty() {
        println!("No active worktrees.");
        return Ok(());
    }

    // Build header from group config
    let agent_headers: String = config
        .group
        .iter()
        .map(|name| format!("{:<10}", name))
        .collect();

    println!("{:<4} {:<30} {}  {:<8} {:<4}", "Q", "Branch", agent_headers, "PR", "Mon");
    println!("{}", "-".repeat(75));

    for q in &quadrants {
        // Show statuses in group order (not arbitrary HashMap order)
        let agent_statuses: String = config
            .group
            .iter()
            .map(|name| {
                let icon = q
                    .agents
                    .get(name)
                    .map(|s| format!("{} {:?}", s.status.icon(&config.status_icons), s.status))
                    .unwrap_or_else(|| "--".into());
                format!("{:<10}", icon)
            })
            .collect();

        println!(
            "{:<4} {:<30} {}  {:<8} {:<4}",
            q.quadrant,
            q.branch,
            agent_statuses,
            q.pr_status.as_deref().unwrap_or("--"),
            q.monitor,
        );
    }

    Ok(())
}

fn fetch_pr_status(store: &mut crate::session::store::SessionStore) -> Result<()> {
    for session in &mut store.sessions {
        for q in &mut session.quadrants {
            let output = std::process::Command::new("gh")
                .args(["pr", "list", "--head", &q.branch, "--json", "number,state", "--limit", "1"])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Parse minimal JSON: [{"number":42,"state":"OPEN"}]
                    if let Ok(prs) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                        if let Some(pr) = prs.first() {
                            let num = pr.get("number").and_then(|n| n.as_u64()).unwrap_or(0);
                            let state = pr.get("state").and_then(|s| s.as_str()).unwrap_or("");
                            let icon = match state {
                                "OPEN" => "",
                                "MERGED" => " merged",
                                "CLOSED" => " closed",
                                _ => "",
                            };
                            q.pr_status = Some(format!("#{}{}", num, icon));
                        }
                    }
                }
            }
        }
    }
    store.save()?;
    Ok(())
}
