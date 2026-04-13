use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::layout::quadrant::WindowBounds;

/// Execute an AppleScript and return Ok on success.
pub fn run(script: &str) -> Result<()> {
    run_with_args(script, &[])
}

/// Execute an AppleScript with argv and return Ok on success.
pub fn run_with_args(script: &str, args: &[&str]) -> Result<()> {
    let output = run_osascript(script, args)?;

    if !output.status.success() {
        anyhow::bail!("{}", format_osascript_error(script, args, &output.stderr));
    }

    Ok(())
}

/// Execute an AppleScript and return its stdout.
pub fn run_capture(script: &str) -> Result<String> {
    run_capture_with_args(script, &[])
}

/// Execute an AppleScript with argv and return its stdout.
pub fn run_capture_with_args(script: &str, args: &[&str]) -> Result<String> {
    let output = run_osascript(script, args)?;

    if !output.status.success() {
        anyhow::bail!("{}", format_osascript_error(script, args, &output.stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn create_window(cwd: &Path, initial_input: Option<&str>) -> String {
    let escaped_cwd = escape_applescript(&cwd.display().to_string());
    let initial_input = initial_input
        .map(applescript_string_literal)
        .map(|payload| format!("\n    set initial input of cfg to {}", payload))
        .unwrap_or_default();
    format!(
        r#"tell application "Ghostty"
    set cfg to new surface configuration
    set initial working directory of cfg to "{}"
{}
    set win to new window with configuration cfg
    set pane to focused terminal of selected tab of win
    set winId to id of win as string
    set paneId to id of pane as string
end tell
return winId & "," & paneId"#,
        escaped_cwd, initial_input,
    )
}

pub fn list_window_ids() -> String {
    r#"tell application "Ghostty"
    set output to {}
    repeat with win in windows
        set end of output to (id of win as string)
    end repeat
    return output as string
end tell"#
        .to_string()
}

pub fn front_window_terminal() -> String {
    r#"tell application "Ghostty"
    set targetTerminal to focused terminal of selected tab of front window
    return id of targetTerminal as string
end tell"#
        .to_string()
}

pub fn place_front_window(bounds: &WindowBounds) -> String {
    format!(
        r#"tell application "System Events"
    tell process "Ghostty"
        set position of front window to {{{}, {}}}
        set size of front window to {{{}, {}}}
    end tell
end tell"#,
        bounds.x, bounds.y, bounds.width, bounds.height,
    )
}

/// Split the current pane in a given direction.
pub fn split_direction(terminal_id: &str, direction: &str, initial_input: Option<&str>) -> String {
    let escaped_terminal_id = escape_applescript(terminal_id);
    let setup = initial_input
        .map(applescript_string_literal)
        .map(|payload| {
            format!(
                "    set cfg to new surface configuration\n    set initial input of cfg to {}\n",
                payload
            )
        })
        .unwrap_or_default();
    let split_expr = if initial_input.is_some() {
        format!(
            "split targetTerminal direction {} with configuration cfg",
            direction
        )
    } else {
        format!("split targetTerminal direction {}", direction)
    };
    format!(
        r#"tell application "Ghostty"
{}
    set targetTerminal to first terminal whose (id as string) is "{}"
    set newTerminal to {}
    return id of newTerminal as string
end tell"#,
        setup, escaped_terminal_id, split_expr,
    )
}

pub fn split_focused_direction(
    _window_id: &str,
    direction: &str,
    initial_input: Option<&str>,
) -> String {
    let setup = initial_input
        .map(applescript_string_literal)
        .map(|payload| {
            format!(
                "    set cfg to new surface configuration\n    set initial input of cfg to {}\n",
                payload
            )
        })
        .unwrap_or_default();
    let split_expr = if initial_input.is_some() {
        format!(
            "split targetTerminal direction {} with configuration cfg",
            direction
        )
    } else {
        format!("split targetTerminal direction {}", direction)
    };
    format!(
        r#"tell application "Ghostty"
{}
    set targetWin to front window
    set targetTerminal to focused terminal of selected tab of targetWin
    set newTerminal to {}
    return id of newTerminal as string
end tell"#,
        setup, split_expr,
    )
}

/// Send text to a specific terminal.
pub fn send_text(terminal_id: &str, text: &str) -> String {
    let escaped_terminal_id = escape_applescript(terminal_id);
    let payload = applescript_string_literal(text);
    format!(
        r#"tell application "Ghostty"
    set targetTerminal to first terminal whose (id as string) is "{}"
    input text {} to targetTerminal
end tell"#,
        escaped_terminal_id, payload,
    )
}

pub fn send_text_to_focused_window(_window_id: &str, text: &str) -> String {
    let payload = applescript_string_literal(text);
    format!(
        r#"tell application "Ghostty"
    set targetWin to front window
    set targetTerminal to focused terminal of selected tab of targetWin
    input text {} to targetTerminal
end tell"#,
        payload,
    )
}

/// Send text to the first terminal in a window matched by title.
pub fn send_text_to_window_script() -> &'static str {
    r#"on run argv
    set windowTitle to item 1 of argv
    set payload to item 2 of argv
    tell application "Ghostty"
        set targetWin to first window whose name contains windowTitle
        input text payload to terminal 1 of selected tab of targetWin
    end tell
end run"#
}

/// Focus a window by id.
pub fn focus_window(window_id: &str) -> String {
    let escaped_window_id = escape_applescript(window_id);
    format!(
        r#"tell application "Ghostty"
    activate
    set targetWin to first window whose (id as string) is "{}"
    set index of targetWin to 1
end tell"#,
        escaped_window_id,
    )
}

/// Focus a window by title.
pub fn focus_window_title(window_title: &str) -> String {
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

/// Close a window by id.
pub fn close_window(window_id: &str) -> String {
    let escaped_window_id = escape_applescript(window_id);
    format!(
        r#"tell application "Ghostty"
    set targetWin to first window whose (id as string) is "{}"
    close targetWin
end tell"#,
        escaped_window_id,
    )
}

/// Close a window by title.
pub fn close_window_title(window_title: &str) -> String {
    let escaped = window_title.replace('"', "\\\"");
    format!(
        r#"tell application "Ghostty"
    set targetWin to first window whose name contains "{}"
    close targetWin
end tell"#,
        escaped,
    )
}

/// Capture pane content from a terminal.
/// Note: Ghostty's AppleScript API for text capture may need adaptation
/// based on the exact object model in your Ghostty version.
pub fn capture_pane(terminal_id: &str) -> String {
    let escaped_terminal_id = escape_applescript(terminal_id);
    format!(
        r#"tell application "Ghostty"
    set targetTerminal to first terminal whose (id as string) is "{}"
    return text of targetTerminal
end tell"#,
        escaped_terminal_id,
    )
}

/// Capture pane content from the first terminal in a window matched by title.
pub fn capture_pane_title(window_title: &str) -> String {
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

pub fn capture_front_window() -> String {
    r#"tell application "Ghostty"
    set targetTerminal to focused terminal of selected tab of front window
    return text of targetTerminal
end tell"#
        .to_string()
}

pub fn front_window_id() -> String {
    r#"tell application "Ghostty"
    return id of front window
end tell"#
        .to_string()
}

fn escape_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn applescript_string_literal(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let parts = normalized
        .split('\n')
        .map(|part| format!("\"{}\"", escape_applescript(part)))
        .collect::<Vec<_>>();

    if parts.is_empty() {
        return "\"\"".to_string();
    }

    parts.join(" & return & ")
}

fn run_osascript(script: &str, args: &[&str]) -> Result<std::process::Output> {
    crate::debug::log(
        "osascript",
        &format!(
            "run argv={} script={:?}",
            if args.is_empty() {
                "[]".to_string()
            } else {
                format!(
                    "[{}]",
                    args.iter()
                        .map(|arg| format!("{:?}", arg))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
            script
        ),
    );
    Command::new("osascript")
        .arg("-e")
        .arg(script)
        .args(args)
        .output()
        .context("Failed to run osascript")
        .inspect(|output| {
            crate::debug::log(
                "osascript",
                &format!(
                    "exit={} stdout={:?} stderr={:?}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            );
        })
}

fn format_osascript_error(script: &str, args: &[&str], stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    if !crate::debug::debug_ghostty() {
        return format!("AppleScript error: {}", stderr);
    }

    let argv = if args.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            args.iter()
                .map(|arg| format!("{:?}", arg))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    format!(
        "AppleScript error: {}\nargv: {}\nscript:\n{}",
        stderr,
        argv,
        numbered_script(script)
    )
}

fn numbered_script(script: &str) -> String {
    script
        .lines()
        .enumerate()
        .map(|(index, line)| format!("{:>2}: {}", index + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::quadrant::WindowBounds;
    use std::path::Path;

    #[test]
    fn test_create_window_script() {
        let script = create_window(Path::new("/tmp/worktree"), None);
        assert!(script.contains("Ghostty"));
        assert!(script.contains("set cfg to new surface configuration"));
        assert!(script.contains("set win to new window with configuration cfg"));
        assert!(script.contains("/tmp/worktree"));
        assert!(script.contains("focused terminal of selected tab of win"));
        assert!(script.contains("return winId & \",\" & paneId"));
    }

    #[test]
    fn test_create_window_escapes_quotes_in_path() {
        let script = create_window(Path::new("/tmp/with\"quote"), None);
        assert!(script.contains(r#"set initial working directory of cfg to "/tmp/with\"quote""#));
    }

    #[test]
    fn test_create_window_with_initial_input() {
        let script = create_window(Path::new("/tmp/worktree"), Some("claude --model opus\n"));
        assert!(script.contains("set initial input of cfg"));
        assert!(script.contains(r#""claude --model opus" & return & """#));
    }

    #[test]
    fn test_place_front_window_script() {
        let bounds = WindowBounds {
            x: 0,
            y: 0,
            width: 960,
            height: 540,
        };
        let script = place_front_window(&bounds);
        assert!(script.contains("System Events"));
        assert!(script.contains("set position of front window to {0, 0}"));
        assert!(script.contains("set size of front window to {960, 540}"));
    }

    #[test]
    fn test_list_window_ids_script() {
        let script = list_window_ids();
        assert!(script.contains("repeat with win in windows"));
        assert!(script.contains("id of win as string"));
    }

    #[test]
    fn test_front_window_terminal_script() {
        let script = front_window_terminal();
        assert!(script.contains("focused terminal of selected tab of front window"));
        assert!(script.contains("return id of targetTerminal as string"));
    }

    #[test]
    fn test_split_direction_script() {
        let script = split_direction("123", "right", None);
        assert!(script.contains(r#"(id as string) is "123""#));
        assert!(script.contains("direction right"));

        let script = split_direction("123", "down", None);
        assert!(script.contains("direction down"));
    }

    #[test]
    fn test_split_direction_with_initial_input_script() {
        let script = split_direction("123", "right", Some("codex\n"));
        assert!(script.contains("set cfg to new surface configuration"));
        assert!(script.contains("with configuration cfg"));
        assert!(script.contains(r#"set initial input of cfg to "codex" & return & """#));
    }

    #[test]
    fn test_split_focused_direction_with_initial_input_script() {
        let script = split_focused_direction("window-1", "right", Some("codex\n"));
        assert!(script.contains("set targetWin to front window"));
        assert!(script.contains("focused terminal of selected tab of targetWin"));
        assert!(script.contains("with configuration cfg"));
    }

    #[test]
    fn test_send_text_escapes_quotes() {
        let script = send_text("123", "echo \"hi\"\n");
        assert!(script.contains(r#"(id as string) is "123""#));
        assert!(script.contains(r#"input text "echo \"hi\"" & return & "" to targetTerminal"#));
    }

    #[test]
    fn test_focus_window_script() {
        let script = focus_window("456");
        assert!(script.contains("activate"));
        assert!(script.contains(r#"(id as string) is "456""#));
        assert!(script.contains("set index"));
    }

    #[test]
    fn test_focus_window_title_script() {
        let script = focus_window_title("seance-q1");
        assert!(script.contains("name contains \"seance-q1\""));
        assert!(script.contains("set index"));
    }

    #[test]
    fn test_close_window_script() {
        let script = close_window("456");
        assert!(script.contains("close targetWin"));
        assert!(script.contains(r#"(id as string) is "456""#));
    }

    #[test]
    fn test_send_text_to_terminal_script() {
        let script = send_text("123", "pwd\n");
        assert!(script.contains(r#"first terminal whose (id as string) is "123""#));
        assert!(script.contains(r#"input text "pwd" & return & "" to targetTerminal"#));
    }

    #[test]
    fn test_send_text_to_focused_window_script() {
        let script = send_text_to_focused_window("window-1", "pwd\n");
        assert!(script.contains("set targetWin to front window"));
        assert!(script.contains("focused terminal of selected tab of targetWin"));
        assert!(script.contains(r#"input text "pwd" & return & "" to targetTerminal"#));
    }

    #[test]
    fn test_send_text_to_window_script() {
        let script = send_text_to_window_script();
        assert!(script.contains("name contains windowTitle"));
        assert!(script.contains("input text payload to terminal 1 of selected tab of targetWin"));
    }

    #[test]
    fn test_capture_pane_script() {
        let script = capture_pane("123");
        assert!(script.contains(r#"(id as string) is "123""#));
        assert!(script.contains("return text"));
    }

    #[test]
    fn test_capture_pane_title_script() {
        let script = capture_pane_title("seance-q1-claude");
        assert!(script.contains("name contains \"seance-q1-claude\""));
        assert!(script.contains("return text"));
    }

    #[test]
    fn test_capture_front_window_script() {
        let script = capture_front_window();
        assert!(script.contains("focused terminal of selected tab of front window"));
        assert!(script.contains("return text of targetTerminal"));
    }

    #[test]
    fn test_close_window_title_script() {
        let script = close_window_title("seance-q2");
        assert!(script.contains("name contains \"seance-q2\""));
        assert!(script.contains("close targetWin"));
    }

    #[test]
    fn test_front_window_id_script() {
        let script = front_window_id();
        assert!(script.contains("front window"));
        assert!(script.contains("return id"));
    }

    #[test]
    fn test_format_osascript_error_includes_script_and_args() {
        crate::debug::set_debug_ghostty(true);
        let message = format_osascript_error(
            "line one\nline two",
            &["abc", "payload"],
            b"123:456: syntax error",
        );
        assert!(message.contains("AppleScript error: 123:456: syntax error"));
        assert!(message.contains(r#"argv: ["abc", "payload"]"#));
        assert!(message.contains(" 1: line one"));
        assert!(message.contains(" 2: line two"));
        crate::debug::set_debug_ghostty(false);
    }

    #[test]
    fn test_applescript_string_literal_handles_multiline_payload() {
        let payload = applescript_string_literal("one\ntwo\n");
        assert_eq!(payload, r#""one" & return & "two" & return & """#);
    }
}
