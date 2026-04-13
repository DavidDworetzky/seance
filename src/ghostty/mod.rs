pub mod applescript;

use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::process::Command;
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
    pub terminal_id: Option<TerminalId>,
}

#[derive(Debug, Clone)]
pub struct DashboardSurface<'a> {
    cwd: &'a Path,
}

impl<'a> DashboardSurface<'a> {
    pub fn new(cwd: &'a Path) -> Self {
        Self { cwd }
    }
}

#[derive(Debug, Clone)]
pub struct SpiritSurface {
    bounds: WindowBounds,
}

impl SpiritSurface {
    pub fn new(bounds: WindowBounds) -> Self {
        Self { bounds }
    }

    pub fn bounds(&self) -> &WindowBounds {
        &self.bounds
    }
}

#[derive(Debug, Clone)]
pub struct SpiritWindowRequest<'a> {
    cwd: &'a Path,
    surface: SpiritSurface,
}

impl<'a> SpiritWindowRequest<'a> {
    pub fn new(cwd: &'a Path, bounds: WindowBounds) -> Self {
        Self {
            cwd,
            surface: SpiritSurface::new(bounds),
        }
    }

    pub fn cwd(&self) -> &'a Path {
        self.cwd
    }

    pub fn surface(&self) -> &SpiritSurface {
        &self.surface
    }
}

/// Ghostty terminal backend using AppleScript (macOS).
pub struct GhosttyBackend;

impl GhosttyBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn launch_dashboard(&self, surface: &DashboardSurface<'_>) -> Result<()> {
        let exe = std::env::current_exe()?;
        let debug_flag = if crate::debug::debug_ghostty() {
            " --debug-ghostty"
        } else {
            ""
        };
        let command = format!("{}{} dashboard --inline", exe.display(), debug_flag);
        let status = Command::new("open")
            .args([
                "-na",
                "Ghostty.app",
                "--args",
                &format!("--command={}", command),
                &format!("--working-directory={}", surface.cwd.display()),
            ])
            .status()
            .context("launching Seance dashboard in Ghostty")?;

        ensure!(
            status.success(),
            "launching Seance dashboard in Ghostty failed"
        );
        Ok(())
    }

    pub fn open_spirit_window(
        &self,
        request: &SpiritWindowRequest<'_>,
        initial_input: Option<&TerminalInput>,
    ) -> Result<GhosttyWindow> {
        let known_windows = self.window_ids().unwrap_or_else(|err| {
            crate::debug::log(
                "ghostty",
                &format!(
                    "window snapshot unavailable before spirit launch error={:#}",
                    err
                ),
            );
            Vec::new()
        });

        crate::debug::log(
            "ghostty",
            &format!(
                "creating Ghostty spirit window cwd={}",
                request.cwd().display()
            ),
        );
        let status = Command::new("open")
            .args([
                "-na",
                "Ghostty.app",
                "--args",
                &format!("--working-directory={}", request.cwd().display()),
            ])
            .status()
            .with_context(|| {
                format!(
                    "opening Ghostty spirit window in {}",
                    request.cwd().display()
                )
            })?;
        ensure!(
            status.success(),
            "opening Ghostty spirit window in {} failed",
            request.cwd().display()
        );

        let window_id = self.wait_for_new_window(&known_windows).with_context(|| {
            format!(
                "opening Ghostty spirit window in {}",
                request.cwd().display()
            )
        })?;
        self.place_window(&window_id, request.surface().bounds());

        if let Some(input) = initial_input {
            self.send_text_to_focused_window(&window_id, input)?;
        }

        Ok(GhosttyWindow {
            window_id,
            terminal_id: None,
        })
    }

    /// Backwards-compatible spirit window alias.
    pub fn create_window(&self, cwd: &Path, bounds: &WindowBounds) -> Result<GhosttyWindow> {
        self.create_window_with_input(cwd, bounds, None)
    }

    /// Backwards-compatible spirit window alias.
    pub fn create_window_with_input(
        &self,
        cwd: &Path,
        bounds: &WindowBounds,
        initial_input: Option<&TerminalInput>,
    ) -> Result<GhosttyWindow> {
        let request = SpiritWindowRequest::new(cwd, bounds.clone());
        self.open_spirit_window(&request, initial_input)
    }

    /// Split the current pane to the right.
    pub fn split_right(&self, window_id: &WindowId) -> Result<TerminalId> {
        self.split_right_with_input(window_id, None)
    }

    pub fn split_right_with_input(
        &self,
        window_id: &WindowId,
        initial_input: Option<&TerminalInput>,
    ) -> Result<TerminalId> {
        let script = applescript::split_window_direction(
            window_id.as_ref(),
            "right",
            initial_input.map(AsRef::as_ref),
        );
        let output = applescript::run_capture(&script)
            .with_context(|| format!("splitting Ghostty window {} to the right", window_id))?;
        TerminalId::new(output)
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
        let script = applescript::split_window_direction(
            window_id.as_ref(),
            "down",
            initial_input.map(AsRef::as_ref),
        );
        let output = applescript::run_capture(&script)
            .with_context(|| format!("splitting Ghostty window {} downward", window_id))?;
        TerminalId::new(output)
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

    fn window_ids(&self) -> Result<Vec<WindowId>> {
        let output = applescript::run_capture(&applescript::window_ids())?;
        parse_window_ids(&output)
    }

    fn wait_for_new_window(&self, known_windows: &[WindowId]) -> Result<WindowId> {
        let known: HashSet<&str> = known_windows.iter().map(AsRef::as_ref).collect();
        for attempt in 1..=20 {
            if let Ok(window_ids) = self.window_ids() {
                if let Some(window_id) = window_ids
                    .into_iter()
                    .find(|window_id| !known.contains(window_id.as_ref()))
                {
                    crate::debug::log(
                        "ghostty",
                        &format!(
                            "detected new spirit window attempt={} window_id={}",
                            attempt, window_id
                        ),
                    );
                    return Ok(window_id);
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        anyhow::bail!("Ghostty did not expose a new spirit window")
    }

    fn place_window(&self, window_id: &WindowId, bounds: &WindowBounds) {
        let script = applescript::place_window(window_id.as_ref(), bounds);

        for attempt in 1..=10 {
            match applescript::run(&script) {
                Ok(()) => {
                    crate::debug::log(
                        "ghostty",
                        &format!(
                            "placed spirit window attempt={} window_id={} bounds={{x:{}, y:{}, width:{}, height:{}}}",
                            attempt, window_id, bounds.x, bounds.y, bounds.width, bounds.height
                        ),
                    );
                    return;
                }
                Err(err) => {
                    crate::debug::log(
                        "ghostty",
                        &format!(
                            "place spirit window skipped attempt={} window_id={} error={:#}",
                            attempt, window_id, err
                        ),
                    );
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

fn parse_window_ids(output: &str) -> Result<Vec<WindowId>> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(WindowId::new)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_window_ids() {
        let ids = parse_window_ids("window-1\nwindow-2\n").unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].as_ref(), "window-1");
        assert_eq!(ids[1].as_ref(), "window-2");
    }

    #[test]
    fn test_parse_window_ids_ignores_blank_lines() {
        let ids = parse_window_ids("\nwindow-1\n\n").unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].as_ref(), "window-1");
    }

    #[test]
    fn test_spirit_window_request_preserves_bounds() {
        let request = SpiritWindowRequest::new(
            Path::new("/tmp/worktree"),
            WindowBounds {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            },
        );
        let bounds = request.surface().bounds();
        assert_eq!(request.cwd(), Path::new("/tmp/worktree"));
        assert_eq!(bounds.x, 1);
        assert_eq!(bounds.y, 2);
        assert_eq!(bounds.width, 3);
        assert_eq!(bounds.height, 4);
    }
}
