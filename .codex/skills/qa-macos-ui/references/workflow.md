# QA Runbook

## Artifact Setup

Use a dedicated artifact directory per run so screenshots and logs stay grouped.

```bash
mkdir -p /tmp/seance-qa
```

Recommended artifact set:

- `/tmp/seance-qa/run.log`
- `/tmp/seance-qa/before.png`
- `/tmp/seance-qa/after.png`
- `/tmp/seance-qa/failure.png`

## Launch Patterns

Basic dashboard launch:

```bash
cargo run -- --debug-ghostty dashboard
```

If the scenario is command-specific, launch that command directly and preserve stderr:

```bash
cargo run -- --debug-ghostty add 2>&1 | tee /tmp/seance-qa/run.log
```

## Accessibility Preflight

Check whether `System Events` can see the target process:

```bash
osascript -e 'tell application "System Events" to count processes'
```

Check whether Ghostty is exposed to Accessibility:

```bash
osascript -e 'tell application "System Events" to tell process "Ghostty" to count windows'
```

Interpretation:

- `not allowed assistive access` or `-1719`: blocked by Accessibility permissions.
- `Invalid index` or zero windows: app launched but no window is currently exposed via AX.

## AppleScript Fundamentals

Prefer app-native AppleScript for app objects:

```applescript
tell application "Ghostty"
    set win to front window
    set tab1 to selected tab of win
    set term1 to focused terminal of tab1
end tell
```

Use `System Events` for generic window operations and key presses:

```applescript
tell application "System Events"
    tell process "Ghostty"
        set frontmost to true
        keystroke "a"
        key code 36
    end tell
end tell
```

Use retries for AX-driven actions. A window may exist in the app before it appears in the Accessibility tree.

## Screenshot Capture

Capture the full screen:

```bash
screencapture -x /tmp/seance-qa/after.png
```

Capture a specific window interactively if needed:

```bash
screencapture -x -w /tmp/seance-qa/window.png
```

If image inspection is available, inspect the file directly. Otherwise still keep the screenshot as an artifact and pair it with logs.

## Log Inspection

Primary local log:

```bash
tail -n 200 ~/Library/Application\ Support/seance/diagnostic.log
```

Useful search patterns:

```bash
rg -n "AppleScript error|add flow step=|launch_agent|split_right|split_down|window ready" \
  ~/Library/Application\ Support/seance/diagnostic.log
```

Common interpretations:

- `add flow step=create_window`: failure before the first usable pane.
- `launch_agent agent=claude`: failure while launching Claude in a pane.
- `AppleScript error`: automation layer failure.
- `window ready`: Ghostty created the window successfully.

## Recommended QA Pattern

1. Launch with diagnostics enabled.
2. Trigger one concrete scenario.
3. Capture a screenshot before interaction if the initial state matters.
4. Drive the app with AppleScript or key events.
5. Capture a screenshot immediately after the expected state should appear.
6. Pull the smallest relevant log excerpt.
7. Decide:
   - `pass`: expected visual state present, no blocking log error.
   - `fail`: wrong visual state or blocking log error.
   - `blocked`: permissions or missing dependency prevent valid execution.

## Verdict Template

Use this structure in the final QA note:

```text
Scenario: <what was tested>
Command: <exact cargo run command>
Artifacts: <screenshot paths and log path>
Observed UI: <what the screenshot shows>
Observed logs: <key lines only>
Verdict: pass | fail | blocked
Reason: <single clear sentence>
```
