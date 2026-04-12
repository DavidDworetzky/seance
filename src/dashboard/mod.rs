pub mod app;
pub mod keymap;
pub mod ui;

use anyhow::Result;
use clap::Args;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::stdout;

use app::App;
use crate::config::schema::Config;
use crate::ghostty::GhosttyBackend;

#[derive(Args, Debug, Clone, Default)]
pub struct DashboardArgs {
    /// Launch the dashboard in a new Ghostty window
    #[arg(long)]
    pub ghostty: bool,

    /// Run in the current terminal even if Ghostty launch is enabled
    #[arg(long, hide = true)]
    pub no_ghostty: bool,
}

pub async fn run_entry(args: DashboardArgs) -> Result<()> {
    let config = Config::load(None)?;
    if should_launch_in_ghostty(&config, &args) {
        launch_in_ghostty()?;
        return Ok(());
    }

    run().await
}

pub async fn run_default() -> Result<()> {
    run_entry(DashboardArgs::default()).await
}

pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new().await?;

    // Main loop
    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                if app.handle_key(key).await? {
                    break;
                }
            }
        }

        // Periodic refresh of spirit status
        app.refresh().await?;
    }

    // Cleanup
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn should_launch_in_ghostty(config: &Config, args: &DashboardArgs) -> bool {
    if args.no_ghostty {
        return false;
    }

    if args.ghostty {
        return true;
    }

    config.dashboard.launch_in_ghostty && !running_inside_ghostty()
}

fn running_inside_ghostty() -> bool {
    std::env::var("TERM_PROGRAM")
        .map(|value| value.eq_ignore_ascii_case("ghostty"))
        .unwrap_or(false)
}

fn launch_in_ghostty() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let exe = std::env::current_exe()?;
    let command = format!(
        "{} dashboard --no-ghostty\n",
        shell_quote(&exe.to_string_lossy())
    );

    let ghostty = GhosttyBackend::new();
    let window = ghostty.create_window_default(&cwd)?;
    ghostty.send_text(&window.terminal_id, &command)?;
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("/tmp/seance"), "'/tmp/seance'");
        assert_eq!(shell_quote("/tmp/it's-seance"), r#"'/tmp/it'"'"'s-seance'"#);
    }
}
