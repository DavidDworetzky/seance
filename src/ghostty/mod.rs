pub mod applescript;

use anyhow::{Context, Result, ensure};
use std::fmt;
use std::path::Path;
use std::time::Duration;

use crate::layout::quadrant::WindowBounds;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(String);

impl WindowId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure!(
            !value.trim().is_empty(),
            "Ghostty window id must not be empty"
        );
        Ok(Self(value))
    }
}

impl AsRef<str> for WindowId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminalId(String);

impl TerminalId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure!(
            !value.trim().is_empty(),
            "Ghostty terminal id must not be empty"
        );
        Ok(Self(value))
    }
}

impl AsRef<str> for TerminalId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TerminalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowTitle(String);

impl WindowTitle {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure!(
            !value.trim().is_empty(),
            "Ghostty window title must not be empty"
        );
        Ok(Self(value))
    }
}

impl AsRef<str> for WindowTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WindowTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalInput(String);

impl TerminalInput {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl AsRef<str> for TerminalInput {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct GhosttyWindow {
    pub window_id: WindowId,
    pub terminal_id: TerminalId,
}

/// Ghostty terminal backend using AppleScript (macOS).
pub struct GhosttyBackend;

impl GhosttyBackend {
    pub fn new() -> Self {
        Self
    }

    /// Create a new Ghostty window at the given path and position.
    pub fn create_window(&self, cwd: &Path, bounds: &WindowBounds) -> Result<GhosttyWindow> {
        self.create_window_with_input(cwd, bounds, None)
    }

    pub fn create_window_with_input(
        &self,
        cwd: &Path,
        bounds: &WindowBounds,
        initial_input: Option<&TerminalInput>,
    ) -> Result<GhosttyWindow> {
        let script = applescript::create_window(cwd, initial_input.map(AsRef::as_ref));
        let output = applescript::run_capture(&script).with_context(|| {
            format!(
                "creating Ghostty window in {} with bounds x={} y={} width={} height={}",
                cwd.display(),
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height
            )
        })?;

        let window = parse_window(output).with_context(|| {
            format!(
                "creating Ghostty window in {} with bounds x={} y={} width={} height={}",
                cwd.display(),
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height
            )
        })?;

        self.place_front_window(bounds);

        Ok(window)
    }
}

impl GhosttyBackend {
    fn place_front_window(&self, bounds: &WindowBounds) {
        let script = applescript::place_front_window(bounds);

        for attempt in 1..=10 {
            match applescript::run(&script) {
                Ok(()) => {
                    crate::debug::log(
                        "ghostty",
                        &format!(
                            "placed front window attempt={} bounds={{x:{}, y:{}, width:{}, height:{}}}",
                            attempt, bounds.x, bounds.y, bounds.width, bounds.height
                        ),
                    );
                    return;
                }
                Err(err) => {
                    crate::debug::log(
                        "ghostty",
                        &format!(
                            "place front window skipped attempt={} error={:#}",
                            attempt, err
                        ),
                    );
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

fn parse_window(output: String) -> Result<GhosttyWindow> {
    let mut parts = output.splitn(2, ',');
    let window_id = parts
        .next()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Ghostty did not return a window id"))?;
    let terminal_id = parts
        .next()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Ghostty did not return a terminal id"))?;

    Ok(GhosttyWindow {
        window_id: WindowId::new(window_id)?,
        terminal_id: TerminalId::new(terminal_id)?,
    })
}

impl GhosttyBackend {
    /// Split the current pane to the right.
    pub fn split_right(&self, window_id: &WindowId) -> Result<TerminalId> {
        self.split_right_with_input(window_id, None)
    }

    pub fn split_right_with_input(
        &self,
        window_id: &WindowId,
        initial_input: Option<&TerminalInput>,
    ) -> Result<TerminalId> {
        let script = applescript::split_focused_direction(window_id.as_ref(), "right", None);
        let output = applescript::run_capture(&script)
            .with_context(|| format!("splitting Ghostty window {} to the right", window_id))?;
        let new_terminal = TerminalId::new(output)?;
        if let Some(input) = initial_input {
            self.send_text_to_focused_window(window_id, input)?;
        }
        Ok(new_terminal)
    }

    /// Split the current pane downward.
    pub fn split_down(&self, window_id: &WindowId) -> Result<TerminalId> {
        self.split_down_with_input(window_id, None)
    }

    pub fn split_down_with_input(
        &self,
        window_id: &WindowId,
        initial_input: Option<&TerminalInput>,
    ) -> Result<TerminalId> {
        let script = applescript::split_focused_direction(window_id.as_ref(), "down", None);
        let output = applescript::run_capture(&script)
            .with_context(|| format!("splitting Ghostty window {} downward", window_id))?;
        let new_terminal = TerminalId::new(output)?;
        if let Some(input) = initial_input {
            self.send_text_to_focused_window(window_id, input)?;
        }
        Ok(new_terminal)
    }

    /// Send text to a specific Ghostty pane.
    pub fn send_text(&self, terminal_id: &TerminalId, text: &TerminalInput) -> Result<()> {
        let script = applescript::send_text(terminal_id.as_ref(), text.as_ref());
        applescript::run(&script)
            .with_context(|| format!("sending text to Ghostty terminal {}", terminal_id))
    }

    pub fn send_text_to_focused_window(
        &self,
        window_id: &WindowId,
        text: &TerminalInput,
    ) -> Result<()> {
        let script = applescript::send_text_to_focused_window(window_id.as_ref(), text.as_ref());
        applescript::run(&script).with_context(|| {
            format!(
                "sending text to focused Ghostty terminal in window {}",
                window_id
            )
        })
    }

    /// Send text to the first pane in a window matched by title.
    pub fn send_text_to_window(
        &self,
        window_title: &WindowTitle,
        text: &TerminalInput,
    ) -> Result<()> {
        applescript::run_with_args(
            applescript::send_text_to_window_script(),
            &[window_title.as_ref(), text.as_ref()],
        )
        .with_context(|| format!("sending text to Ghostty window {}", window_title))
    }

    /// Focus a window by id.
    pub fn focus_window(&self, window_id: &WindowId) -> Result<()> {
        let script = applescript::focus_window(window_id.as_ref());
        applescript::run(&script)
    }

    /// Focus a window by title.
    pub fn focus_window_title(&self, window_title: &WindowTitle) -> Result<()> {
        let script = applescript::focus_window_title(window_title.as_ref());
        applescript::run(&script)
    }

    /// Close a window by id.
    pub fn close_window(&self, window_id: &WindowId) -> Result<()> {
        let script = applescript::close_window(window_id.as_ref());
        applescript::run(&script)
    }

    /// Close a window by title.
    pub fn close_window_title(&self, window_title: &WindowTitle) -> Result<()> {
        let script = applescript::close_window_title(window_title.as_ref());
        applescript::run(&script)
    }

    /// Capture terminal output from a pane.
    pub fn capture_pane(&self, terminal_id: &TerminalId) -> Result<String> {
        let script = applescript::capture_pane(terminal_id.as_ref());
        applescript::run_capture(&script)
    }

    /// Capture terminal output from the first pane in a window matched by title.
    pub fn capture_pane_title(&self, window_title: &WindowTitle) -> Result<String> {
        let script = applescript::capture_pane_title(window_title.as_ref());
        applescript::run_capture(&script)
    }

    pub fn front_window_id(&self) -> Result<WindowId> {
        let output = applescript::run_capture(&applescript::front_window_id())?;
        WindowId::new(output)
    }
}
