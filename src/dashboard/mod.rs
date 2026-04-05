pub mod app;
pub mod keymap;
pub mod ui;

use anyhow::Result;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::stdout;

use app::App;

pub async fn run_in_ghostty() -> Result<()> {
    let exe = std::env::current_exe()?;
    let cwd = std::env::current_dir()?;

    let result = std::process::Command::new("open")
        .args([
            "-na",
            "Ghostty.app",
            "--args",
            "-e",
            &format!("{} dashboard --inline", exe.display()),
            &format!("--working-directory={}", cwd.display()),
        ])
        .status();

    match result {
        Ok(status) if status.success() => Ok(()),
        _ => {
            // Ghostty not available, fall back to inline TUI
            run().await
        }
    }
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
