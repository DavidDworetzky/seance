pub mod claude;
pub mod codex;
pub mod prompt;

use anyhow::{Context, Result};

use crate::config::schema::{AgentConfig, Config, PromptInjection};

/// Build the full command string for launching an agent with optional prompt.
pub fn build_launch_command(agent_config: &AgentConfig, prompt_file: Option<&str>) -> String {
    let base = &agent_config.command;

    match (&agent_config.prompt_injection, prompt_file) {
        (_, None) => base.clone(),
        (PromptInjection::Trailing, Some(pf)) => {
            format!("{} -- \"$(cat {})\"", base, pf)
        }
        (PromptInjection::Flag, Some(pf)) => {
            format!("{} --prompt \"$(cat {})\"", base, pf)
        }
        (PromptInjection::File, Some(_pf)) => {
            // File-based injection writes the prompt to a well-known path
            // before launching. The agent picks it up from there.
            base.clone()
        }
    }
}

pub fn generate_branch_name(config: &Config, prompt: &str) -> Result<String> {
    let auto_name_command = config
        .auto_name
        .command
        .clone()
        .or_else(|| {
            config
                .agents
                .get(&config.auto_name.agent)
                .and_then(|agent| agent.auto_name_command.clone())
        })
        .unwrap_or_else(|| claude::AUTO_NAME_COMMAND.to_string());

    let request = format!(
        "Generate a short git branch name for this task. Return only the branch name, using lowercase kebab-case and no prefix text.\n\n{}",
        prompt.trim()
    );
    let command = format!("{} {}", auto_name_command, shell_quote(&request));
    let output = std::process::Command::new("sh")
        .args(["-lc", &command])
        .output()
        .with_context(|| "running auto-name command")?;

    if !output.status.success() {
        anyhow::bail!(
            "Auto-name command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = String::from_utf8(output.stdout)?;
    let raw = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("seance");

    Ok(sanitize_branch_name(raw))
}

fn sanitize_branch_name(name: &str) -> String {
    let sanitized = name
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .chars()
        .map(|ch| match ch {
            'a'..='z' | '0'..='9' | '-' | '/' => ch,
            'A'..='Z' => ch.to_ascii_lowercase(),
            ' ' | '_' => '-',
            _ => '-',
        })
        .collect::<String>();

    sanitized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_matches('-'))
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
        .trim_matches('-')
        .to_string()
        .chars()
        .take(64)
        .collect::<String>()
        .if_empty_then("seance")
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r#"'\''"#))
}

trait IfEmptyThen {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl IfEmptyThen for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{AgentConfig, AutoNameConfig};

    #[test]
    fn test_build_launch_no_prompt() {
        let ac = AgentConfig {
            command: "claude".into(),
            prompt_injection: PromptInjection::Trailing,
            auto_name: false,
            auto_name_command: None,
        };
        assert_eq!(build_launch_command(&ac, None), "claude");
    }

    #[test]
    fn test_build_launch_trailing_prompt() {
        let ac = AgentConfig {
            command: "claude".into(),
            prompt_injection: PromptInjection::Trailing,
            auto_name: false,
            auto_name_command: None,
        };
        let result = build_launch_command(&ac, Some("/tmp/prompt.md"));
        assert_eq!(result, "claude -- \"$(cat /tmp/prompt.md)\"");
    }

    #[test]
    fn test_build_launch_flag_prompt() {
        let ac = AgentConfig {
            command: "codex".into(),
            prompt_injection: PromptInjection::Flag,
            auto_name: false,
            auto_name_command: None,
        };
        let result = build_launch_command(&ac, Some("/tmp/prompt.md"));
        assert_eq!(result, "codex --prompt \"$(cat /tmp/prompt.md)\"");
    }

    #[test]
    fn test_build_launch_file_prompt() {
        let ac = AgentConfig {
            command: "cursor".into(),
            prompt_injection: PromptInjection::File,
            auto_name: false,
            auto_name_command: None,
        };
        // File injection doesn't modify the command
        let result = build_launch_command(&ac, Some("/tmp/prompt.md"));
        assert_eq!(result, "cursor");
    }

    #[test]
    fn test_sanitize_branch_name() {
        assert_eq!(sanitize_branch_name("Fix Login Flow"), "fix-login-flow");
        assert_eq!(
            sanitize_branch_name("\"feat/api cleanup\""),
            "feat/api-cleanup"
        );
        assert_eq!(sanitize_branch_name(""), "seance");
    }

    #[test]
    fn test_shell_quote() {
        assert_eq!(shell_quote("hello"), "'hello'");
        assert_eq!(shell_quote("don't"), "'don'\\''t'");
    }

    #[test]
    fn test_generate_branch_name_uses_default_command() {
        let config = Config {
            auto_name: AutoNameConfig {
                enabled: true,
                agent: "claude".into(),
                command: Some("printf fix-login".into()),
            },
            ..Config::default()
        };

        let branch = generate_branch_name(&config, "Fix login").unwrap();
        assert_eq!(branch, "fix-login");
    }
}
