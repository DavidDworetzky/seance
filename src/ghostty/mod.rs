pub mod applescript;

use anyhow::Result;
use std::path::Path;

use crate::layout::quadrant::WindowBounds;

/// Ghostty terminal backend using AppleScript (macOS).
pub struct GhosttyBackend;

impl GhosttyBackend {
    pub fn new() -> Self {
        Self
    }

    /// Create a new Ghostty window at the given path and position.
    pub fn create_window(&self, cwd: &Path, bounds: &WindowBounds) -> Result<()> {
        let script = applescript::create_window(cwd, bounds);
        applescript::run(&script)
    }

    /// Split the current pane to the right.
    pub fn split_right(&self) -> Result<()> {
        let script = applescript::split_direction("right");
        applescript::run(&script)
    }

    /// Split the current pane downward.
    pub fn split_down(&self) -> Result<()> {
        let script = applescript::split_direction("down");
        applescript::run(&script)
    }

    /// Send text to the frontmost Ghostty pane.
    pub fn send_text(&self, text: &str) -> Result<()> {
        let script = applescript::send_text(text);
        applescript::run(&script)
    }

    /// Send text to a specific window identified by title.
    pub fn send_text_to_window(&self, window_title: &str, text: &str) -> Result<()> {
        let script = applescript::send_text_to_window(window_title, text);
        applescript::run(&script)
    }

    /// Focus a window by title.
    pub fn focus_window(&self, window_title: &str) -> Result<()> {
        let script = applescript::focus_window(window_title);
        applescript::run(&script)
    }

    /// Close a window by title.
    pub fn close_window(&self, window_title: &str) -> Result<()> {
        let script = applescript::close_window(window_title);
        applescript::run(&script)
    }

    /// Capture terminal output from a window.
    pub fn capture_pane(&self, window_title: &str) -> Result<String> {
        let script = applescript::capture_pane(window_title);
        applescript::run_capture(&script)
    }
}
