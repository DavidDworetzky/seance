pub mod applescript;

use anyhow::Result;
use std::path::Path;

use crate::layout::quadrant::WindowBounds;

#[derive(Debug, Clone)]
pub struct GhosttyWindow {
    pub window_id: String,
    pub terminal_id: String,
}

/// Ghostty terminal backend using AppleScript (macOS).
pub struct GhosttyBackend;

impl GhosttyBackend {
    pub fn new() -> Self {
        Self
    }

    /// Create a new Ghostty window at the given path and position.
    pub fn create_window(&self, cwd: &Path, bounds: &WindowBounds) -> Result<GhosttyWindow> {
        let script = applescript::create_window(cwd, bounds);
        let output = applescript::run_capture(&script)?;
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
            window_id: window_id.to_string(),
            terminal_id: terminal_id.to_string(),
        })
    }

    /// Split the current pane to the right.
    pub fn split_right(&self, terminal_id: &str) -> Result<String> {
        let script = applescript::split_direction(terminal_id, "right");
        applescript::run_capture(&script)
    }

    /// Split the current pane downward.
    pub fn split_down(&self, terminal_id: &str) -> Result<String> {
        let script = applescript::split_direction(terminal_id, "down");
        applescript::run_capture(&script)
    }

    /// Send text to a specific Ghostty pane.
    pub fn send_text(&self, terminal_id: &str, text: &str) -> Result<()> {
        let script = applescript::send_text(terminal_id, text);
        applescript::run(&script)
    }

    /// Focus a window by id.
    pub fn focus_window(&self, window_id: &str) -> Result<()> {
        let script = applescript::focus_window(window_id);
        applescript::run(&script)
    }

    /// Close a window by id.
    pub fn close_window(&self, window_id: &str) -> Result<()> {
        let script = applescript::close_window(window_id);
        applescript::run(&script)
    }

    /// Capture terminal output from a pane.
    pub fn capture_pane(&self, terminal_id: &str) -> Result<String> {
        let script = applescript::capture_pane(terminal_id);
        applescript::run_capture(&script)
    }

    pub fn front_window_id(&self) -> Result<String> {
        applescript::run_capture(&applescript::front_window_id())
    }
}
