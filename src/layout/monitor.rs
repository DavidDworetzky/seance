/// Display information.
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Detect connected displays.
/// Uses AppleScript to query screen bounds on macOS.
/// Falls back to a sensible default if detection fails.
pub fn detect_displays() -> Vec<DisplayInfo> {
    match detect_via_applescript() {
        Ok(displays) if !displays.is_empty() => displays,
        _ => {
            // Fallback: assume a single 1920x1080 display
            vec![DisplayInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }]
        }
    }
}

/// Create a DisplayInfo with specific dimensions (for testing or manual config).
pub fn manual_display(x: i32, y: i32, width: i32, height: i32) -> DisplayInfo {
    DisplayInfo { x, y, width, height }
}

fn detect_via_applescript() -> anyhow::Result<Vec<DisplayInfo>> {
    // Use Finder's desktop bounds as a proxy for display geometry.
    // Each desktop corresponds to a screen.
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(
            r#"tell application "Finder"
    set displayList to ""
    repeat with d in (every desktop)
        set b to bounds of d
        set displayList to displayList & (item 1 of b) & "," & (item 2 of b) & "," & (item 3 of b) & "," & (item 4 of b) & "\n"
    end repeat
    return displayList
end tell"#,
        )
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to detect displays via AppleScript");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut displays = Vec::new();

    for line in stdout.trim().lines() {
        let parts: Vec<i32> = line
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if parts.len() == 4 {
            displays.push(DisplayInfo {
                x: parts[0],
                y: parts[1],
                width: parts[2] - parts[0],
                height: parts[3] - parts[1],
            });
        }
    }

    Ok(displays)
}
