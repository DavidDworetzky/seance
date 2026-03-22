pub mod claude;
pub mod codex;
pub mod prompt;

use crate::config::schema::{AgentConfig, PromptInjection};

/// Build the full command string for launching an agent with optional prompt.
pub fn build_launch_command(
    agent_config: &AgentConfig,
    prompt_file: Option<&str>,
) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::AgentConfig;

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
}
