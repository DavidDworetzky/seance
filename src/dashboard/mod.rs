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
    let backend = crate::ghostty::GhosttyBackend::new();
    let cwd = std::env::current_dir()?;
    let bounds = crate::layout::quadrant::WindowBounds {
        x: 100,
        y: 100,
        width: 1200,
        height: 800,
    };

    match backend.create_window(&cwd, &bounds) {
        Ok(window) => {
            let exe = std::env::current_exe()?
                .to_string_lossy()
                .to_string();
            backend.send_text(
                &window.terminal_id,
                &format!("{} dashboard --inline\n", exe),
            )?;
            Ok(())
        }
        Err(_) => {
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
