use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Open config in $EDITOR
    Edit,
    /// Print resolved config
    Show,
}

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".into());
            let config_path = std::env::current_dir()?.join(".seance.yaml");

            if !config_path.exists() {
                anyhow::bail!("No .seance.yaml found. Run `seance init` first.");
            }

            std::process::Command::new(editor)
                .arg(&config_path)
                .status()?;
        }
        ConfigCommand::Show => {
            let config = crate::config::schema::Config::load(None)?;
            let yaml = serde_yml::to_string(&config)?;
            println!("{}", yaml);
        }
    }

    Ok(())
}
