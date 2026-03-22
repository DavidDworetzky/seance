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
    set pane to selected terminal of selected tab of win
    return (id of win as string) & "," & (id of pane as string)
end tell"#,
        cwd.display(),
        bounds.x,
        bounds.y,
        bounds.x + bounds.width,
        bounds.y + bounds.height,
    )
}

/// Split the current pane in a given direction.
pub fn split_direction(terminal_id: &str, direction: &str) -> String {
    format!(
        r#"tell application "Ghostty"
    set targetTerminal to first terminal whose id is {}
    set newTerminal to split targetTerminal direction {}
    return id of newTerminal as string
end tell"#,
        terminal_id, direction,
    )
}

/// Send text to a specific terminal.
pub fn send_text(terminal_id: &str, text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    set targetTerminal to first terminal whose id is {}
    input text "{}" to targetTerminal
end tell"#,
        terminal_id, escaped,
    )
}

/// Focus a window by id.
pub fn focus_window(window_id: &str) -> String {
    format!(
        r#"tell application "Ghostty"
    activate
    set targetWin to first window whose id is {}
    set index of targetWin to 1
end tell"#,
        window_id,
    )
}

/// Close a window by id.
pub fn close_window(window_id: &str) -> String {
    format!(
        r#"tell application "Ghostty"
    set targetWin to first window whose id is {}
    close targetWin
end tell"#,
        window_id,
    )
}

/// Capture pane content from a terminal.
/// Note: Ghostty's AppleScript API for text capture may need adaptation
/// based on the exact object model in your Ghostty version.
pub fn capture_pane(terminal_id: &str) -> String {
    format!(
        r#"tell application "Ghostty"
    set targetTerminal to first terminal whose id is {}
    return text of targetTerminal
end tell"#,
        terminal_id,
    )
}

pub fn front_window_id() -> String {
    r#"tell application "Ghostty"
    return id of front window as string
end tell"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::quadrant::WindowBounds;
    use std::path::Path;

    #[test]
    fn test_create_window_script() {
        let bounds = WindowBounds {
            x: 0,
            y: 0,
            width: 960,
            height: 540,
        };
        let script = create_window(Path::new("/tmp/worktree"), &bounds);
        assert!(script.contains("Ghostty"));
        assert!(script.contains("/tmp/worktree"));
        assert!(script.contains("0, 0, 960, 540"));
        assert!(script.contains("return (id of win as string)"));
    }

    #[test]
    fn test_split_direction_script() {
        let script = split_direction("123", "right");
        assert!(script.contains("id is 123"));
        assert!(script.contains("direction right"));

        let script = split_direction("123", "down");
        assert!(script.contains("direction down"));
    }

    #[test]
    fn test_send_text_escapes_quotes() {
        let script = send_text("123", "say \"hello\"");
        assert!(script.contains("id is 123"));
        assert!(script.contains(r#"say \"hello\""#));
    }

    #[test]
    fn test_focus_window_script() {
        let script = focus_window("456");
        assert!(script.contains("activate"));
        assert!(script.contains("id is 456"));
        assert!(script.contains("set index"));
    }

    #[test]
    fn test_close_window_script() {
        let script = close_window("456");
        assert!(script.contains("close targetWin"));
        assert!(script.contains("id is 456"));
    }

    #[test]
    fn test_send_text_to_terminal_script() {
        let script = send_text("123", "hello world");
        assert!(script.contains("id is 123"));
        assert!(script.contains("hello world"));
    }

    #[test]
    fn test_capture_pane_script() {
        let script = capture_pane("123");
        assert!(script.contains("id is 123"));
        assert!(script.contains("return text"));
    }

    #[test]
    fn test_front_window_id_script() {
        let script = front_window_id();
        assert!(script.contains("front window"));
        assert!(script.contains("return id"));
    }
}
