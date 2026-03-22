use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct CheckoutArgs {
    /// GitHub PR number
    #[arg(long)]
    pub pr: u64,

    /// Target quadrant
    #[arg(short, long)]
    pub quadrant: Option<u8>,

    /// Target monitor
    #[arg(short, long, default_value = "0")]
    pub monitor: u8,
}

pub async fn run(args: CheckoutArgs) -> Result<()> {
    // Use gh CLI to get PR branch
    let output = std::process::Command::new("gh")
        .args(["pr", "view", &args.pr.to_string(), "--json", "headRefName", "-q", ".headRefName"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to get PR branch: {}", String::from_utf8_lossy(&output.stderr));
    }

    let branch = String::from_utf8(output.stdout)?.trim().to_string();
    println!("Checking out PR #{} (branch: {})", args.pr, branch);

    // Delegate to add with the PR branch
    let add_args = crate::command::add::AddArgs {
        branch: Some(branch),
        prompt: None,
        prompt_file: None,
        agent: None,
        quadrant: args.quadrant,
        monitor: args.monitor,
        base: None,
        auto_name: false,
        no_file_ops: false,
        circle: false,
    };

    crate::command::add::run(add_args).await
}
