---
name: qa-macos-ui
description: Use when validating Seance or another macOS UI flow from `cargo run`, especially when the test requires AppleScript, `System Events`, Accessibility permissions, screenshots, and diagnostic logs to decide pass or fail.
---

# macOS UI QA

Use this skill for manual-but-agentic QA of the app from a local dev build.

## Scope

- Launch the app from source with `cargo run`.
- Drive the UI with AppleScript and `System Events`.
- Capture screenshots and diagnostic logs as evidence.
- Decide pass/fail from both visual state and logs.

## Workflow

1. Create an artifact folder for the run.
2. Start the app with `cargo run`, usually with diagnostics enabled.
3. Verify Accessibility and Automation before depending on `System Events`.
4. Drive the scenario with AppleScript or shell commands.
5. Capture screenshots at the key checkpoints.
6. Read the diagnostic log and stderr from the run.
7. Mark the scenario `pass` only if the UI state matches the expectation and the logs do not show a blocking failure.

## Required Evidence

- Command used to launch the app.
- At least one screenshot of the state under test.
- Relevant log excerpt.
- Explicit pass/fail verdict with the observed reason.

## Default Commands

- Launch:
  `cargo run -- --debug-ghostty dashboard`
- Log file:
  `~/Library/Application Support/seance/diagnostic.log`
- Screenshot:
  `screencapture -x /tmp/seance-qa/step.png`

## Accessibility Rules

- If `System Events` returns `not allowed assistive access` or another `-1719` error, treat that as an environment blocker, not an app pass.
- If AppleScript can launch the app but not manipulate windows, continue collecting screenshots and logs and report the blocker precisely.
- Prefer app-native AppleScript first. Use `System Events` only for window placement, key presses, clicks, and UI tree inspection.

## Pass/Fail Rules

Mark `pass` only when both are true:

- The screenshot or screenshots show the expected state.
- The logs do not contain a blocking runtime error for the tested path.

Mark `fail` when any of these are true:

- The expected UI state is absent or visibly wrong.
- The logs show a blocking error such as `AppleScript error`, `add flow step=...`, panic, crash, or a command that never launches.
- The flow only succeeds because the skill skipped the feature under test.

Mark `blocked` when the environment prevents a valid run:

- Missing Accessibility permission.
- Missing Automation permission.
- Missing external dependency such as Ghostty, Claude, or Codex binaries.

## References

- For the detailed runbook and reusable command snippets, read [references/workflow.md](references/workflow.md).
