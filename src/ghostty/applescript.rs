use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::layout::quadrant::WindowBounds;

/// Execute an AppleScript and return Ok on success.
pub fn run(script: &str) -> Result<()> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("Failed to run osascript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("AppleScript error: {}", stderr.trim());
    }

    Ok(())
}

/// Execute an AppleScript and return its stdout.
pub fn run_capture(script: &str) -> Result<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .context("Failed to run osascript")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("AppleScript error: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Create a new Ghostty window with a specific working directory and bounds.
pub fn create_window(cwd: &Path, bounds: &WindowBounds) -> String {
    format!(
        r#"tell application "Ghostty"
    set cfg to new surface configuration
    set initial working directory of cfg to "{}"
    set win to new window with configuration cfg
    set bounds of win to {{{}, {}, {}, {}}}
end tell"#,
        cwd.display(),
        bounds.x,
        bounds.y,
        bounds.x + bounds.width,
        bounds.y + bounds.height,
    )
}

/// Split the current pane in a given direction.
pub fn split_direction(direction: &str) -> String {
    format!(
        r#"tell application "Ghostty"
    set currentTab to selected tab of front window
    set currentTerminal to terminal 1 of currentTab
    split currentTerminal direction {}
end tell"#,
        direction,
    )
}

/// Send text to the current Ghostty pane.
pub fn send_text(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    input text "{}" to terminal 1 of selected tab of front window
end tell"#,
        escaped,
    )
}

/// Send text to a specific window by title.
pub fn send_text_to_window(window_title: &str, text: &str) -> String {
    let escaped_text = text.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_title = window_title.replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    set targetWin to first window whose name contains "{}"
    input text "{}" to terminal 1 of selected tab of targetWin
end tell"#,
        escaped_title, escaped_text,
    )
}

/// Focus a window by title.
pub fn focus_window(window_title: &str) -> String {
    let escaped = window_title.replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    activate
    set targetWin to first window whose name contains "{}"
    set index of targetWin to 1
end tell"#,
        escaped,
    )
}

/// Close a window by title.
pub fn close_window(window_title: &str) -> String {
    let escaped = window_title.replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    set targetWin to first window whose name contains "{}"
    close targetWin
end tell"#,
        escaped,
    )
}

/// Capture pane content from a window.
/// Note: Ghostty's AppleScript API for text capture may need adaptation
/// based on the exact object model in your Ghostty version.
pub fn capture_pane(window_title: &str) -> String {
    let escaped = window_title.replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    set targetWin to first window whose name contains "{}"
    set targetTerminal to terminal 1 of selected tab of targetWin
    return text of targetTerminal
end tell"#,
        escaped,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::quadrant::WindowBounds;
    use std::path::Path;

    #[test]
    fn test_create_window_script() {
        let bounds = WindowBounds { x: 0, y: 0, width: 960, height: 540 };
        let script = create_window(Path::new("/tmp/worktree"), &bounds);
        assert!(script.contains("Ghostty"));
        assert!(script.contains("/tmp/worktree"));
        assert!(script.contains("0, 0, 960, 540"));
    }

    #[test]
    fn test_split_direction_script() {
        let script = split_direction("right");
        assert!(script.contains("direction right"));

        let script = split_direction("down");
        assert!(script.contains("direction down"));
    }

    #[test]
    fn test_send_text_escapes_quotes() {
        let script = send_text("say \"hello\"");
        assert!(script.contains(r#"say \"hello\""#));
    }

    #[test]
    fn test_focus_window_script() {
        let script = focus_window("seance-q1");
        assert!(script.contains("activate"));
        assert!(script.contains("seance-q1"));
        assert!(script.contains("set index"));
    }

    #[test]
    fn test_close_window_script() {
        let script = close_window("seance-q2");
        assert!(script.contains("close targetWin"));
        assert!(script.contains("seance-q2"));
    }

    #[test]
    fn test_send_text_to_window_script() {
        let script = send_text_to_window("seance-q1-claude", "hello world");
        assert!(script.contains("seance-q1-claude"));
        assert!(script.contains("hello world"));
    }

    #[test]
    fn test_capture_pane_script() {
        let script = capture_pane("seance-q1-claude");
        assert!(script.contains("seance-q1-claude"));
        assert!(script.contains("return text"));
    }
}
