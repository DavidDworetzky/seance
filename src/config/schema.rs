use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::{claude, codex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub main_branch: String,
    pub base_branch: Option<String>,
    pub worktree_dir: String,

    /// Agents per worktree — all share the same worktree, each gets a pane
    pub group: Vec<String>,

    /// Agent-specific configuration
    pub agents: HashMap<String, AgentConfig>,

    /// Split ratios
    pub split_ratio: SplitRatioConfig,

    /// Post-create hooks (run after worktree + file ops)
    pub post_create: Vec<String>,

    /// Pre-merge hooks (run before merge, abort on failure)
    pub pre_merge: Vec<String>,

    /// File operations
    pub files: FileConfig,

    /// Merge strategy
    pub merge_strategy: MergeStrategy,

    /// Session defaults
    pub session: SessionConfig,

    /// Dashboard behavior
    pub dashboard: DashboardConfig,

    /// TUI theme
    pub theme: String,

    /// Status icons
    pub status_icons: StatusIcons,

    /// Monitor layout
    pub monitors: MonitorConfig,

    /// PR integration
    pub pr: PrConfig,

    /// Auto branch naming
    pub auto_name: AutoNameConfig,

    /// Local development toggles
    pub dev: DevConfig,

    /// Quadrants per monitor
    pub quadrants_per_monitor: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            main_branch: "main".into(),
            base_branch: None,
            worktree_dir: "../{project}__seance".into(),
            group: vec!["claude".into(), "codex".into()],
            agents: HashMap::from([
                (
                    "claude".into(),
                    AgentConfig {
                        command: claude::COMMAND.into(),
                        prompt_injection: PromptInjection::Trailing,
                        auto_name: true,
                        auto_name_command: Some(claude::AUTO_NAME_COMMAND.into()),
                    },
                ),
                (
                    "codex".into(),
                    AgentConfig {
                        command: codex::COMMAND.into(),
                        prompt_injection: PromptInjection::Flag,
                        auto_name: false,
                        auto_name_command: None,
                    },
                ),
            ]),
            split_ratio: SplitRatioConfig::default(),
            post_create: vec![],
            pre_merge: vec![],
            files: FileConfig::default(),
            merge_strategy: MergeStrategy::Squash,
            session: SessionConfig::default(),
            dashboard: DashboardConfig::default(),
            theme: "dark".into(),
            status_icons: StatusIcons::default(),
            monitors: MonitorConfig::default(),
            pr: PrConfig::default(),
            auto_name: AutoNameConfig::default(),
            dev: DevConfig::default(),
            quadrants_per_monitor: 4,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.main_branch.trim().is_empty(),
            "config.main_branch must not be empty"
        );
        ensure!(
            !self.worktree_dir.trim().is_empty(),
            "config.worktree_dir must not be empty"
        );
        ensure!(!self.group.is_empty(), "config.group must not be empty");
        ensure!(!self.agents.is_empty(), "config.agents must not be empty");
        ensure!(
            self.split_ratio.agents > 0.0 && self.split_ratio.agents < 1.0,
            "config.split_ratio.agents must be between 0 and 1"
        );
        ensure!(
            self.pr.check_interval > 0,
            "config.pr.check_interval must be greater than 0"
        );
        ensure!(
            self.quadrants_per_monitor > 0,
            "config.quadrants_per_monitor must be greater than 0"
        );

        let mut seen = std::collections::HashSet::new();
        for agent_name in &self.group {
            ensure!(
                seen.insert(agent_name),
                "config.group contains duplicate agent '{}'",
                agent_name
            );
            ensure!(
                self.agents.contains_key(agent_name),
                "config.group references unknown agent '{}'",
                agent_name
            );
        }

        for (agent_name, agent) in &self.agents {
            ensure!(
                !agent_name.trim().is_empty(),
                "config.agents contains an empty agent name"
            );
            ensure!(
                !agent.command.trim().is_empty(),
                "config.agents.{}.command must not be empty",
                agent_name
            );
            if let Some(command) = &agent.auto_name_command {
                ensure!(
                    !command.trim().is_empty(),
                    "config.agents.{}.auto_name_command must not be empty when set",
                    agent_name
                );
            }
        }

        if self.auto_name.enabled {
            ensure!(
                self.agents.contains_key(&self.auto_name.agent),
                "config.auto_name.agent references unknown agent '{}'",
                self.auto_name.agent
            );
        }

        if let Some(command) = &self.auto_name.command {
            ensure!(
                !command.trim().is_empty(),
                "config.auto_name.command must not be empty when set"
            );
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentConfig {
    pub command: String,
    pub prompt_injection: PromptInjection,
    pub auto_name: bool,
    pub auto_name_command: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            prompt_injection: PromptInjection::Trailing,
            auto_name: false,
            auto_name_command: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptInjection {
    /// Append as trailing args: -- "$(cat PROMPT.md)"
    Trailing,
    /// Use a flag: --prompt "$(cat PROMPT.md)"
    Flag,
    /// Write to a file (for IDE-based agents)
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Merge,
    Rebase,
    Squash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SplitRatioConfig {
    /// Agent panes vs shell pane (vertical split)
    pub agents: f64,
}

impl Default for SplitRatioConfig {
    fn default() -> Self {
        Self { agents: 0.8 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    pub copy: Vec<String>,
    pub symlink: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SessionConfig {
    /// Auto-sleep after this duration of inactivity (e.g., "4h")
    pub auto_sleep_after: Option<String>,
    /// Lines of terminal output to capture on sleep
    pub max_terminal_capture: usize,
    pub persist_shell_history: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_sleep_after: None,
            max_terminal_capture: 500,
            persist_shell_history: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DashboardConfig {
    pub launch_in_ghostty: bool,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            launch_in_ghostty: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusIcons {
    pub starting: String,
    pub working: String,
    pub waiting: String,
    pub done: String,
    pub closed: String,
}

impl Default for StatusIcons {
    fn default() -> Self {
        Self {
            starting: "◌".into(),
            working: "⚡".into(),
            waiting: "◎".into(),
            done: "✓".into(),
            closed: "✗".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MonitorConfig {
    pub auto_detect: bool,
    /// Pixel gap between quadrant windows
    pub gap: u32,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            gap: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrConfig {
    pub auto_check: bool,
    /// Seconds between PR status polls
    pub check_interval: u64,
}

impl Default for PrConfig {
    fn default() -> Self {
        Self {
            auto_check: true,
            check_interval: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AutoNameConfig {
    pub enabled: bool,
    pub agent: String,
    pub command: Option<String>,
}

impl Default for AutoNameConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            agent: "claude".into(),
            command: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct DevConfig {
    pub diagnostic_mode: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        config.validate().unwrap();
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.group, vec!["claude", "codex"]);
        assert_eq!(config.quadrants_per_monitor, 4);
        assert!(config.agents.contains_key("claude"));
        assert!(config.agents.contains_key("codex"));
    }

    #[test]
    fn test_default_agents_config() {
        let config = Config::default();
        let claude_agent = config.agents.get("claude").unwrap();
        assert_eq!(claude_agent.command, claude::COMMAND);
        assert!(claude_agent.auto_name);

        let codex_agent = config.agents.get("codex").unwrap();
        assert_eq!(codex_agent.command, codex::COMMAND);
        assert!(!codex_agent.auto_name);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config::default();
        let yaml = serde_yml::to_string(&config).unwrap();
        let parsed: Config = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(parsed.main_branch, config.main_branch);
        assert_eq!(parsed.group, config.group);
    }

    #[test]
    fn test_parse_minimal_yaml() {
        let yaml = "main_branch: develop\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        config.validate().unwrap();
        assert_eq!(config.main_branch, "develop");
        // Defaults should fill in
        assert_eq!(config.group, vec!["claude", "codex"]);
    }

    #[test]
    fn test_parse_custom_group() {
        let yaml = r#"
group:
  - claude
  - codex
  - gemini
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.group, vec!["claude", "codex", "gemini"]);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_unknown_group_agent() {
        let yaml = r#"
group:
  - claude
  - gemini
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("unknown agent 'gemini'"));
    }

    #[test]
    fn test_merge_strategy_serde() {
        let yaml = "merge_strategy: rebase\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(matches!(config.merge_strategy, MergeStrategy::Rebase));
    }

    #[test]
    fn test_status_icons_defaults() {
        let icons = StatusIcons::default();
        assert_eq!(icons.working, "⚡");
        assert_eq!(icons.done, "✓");
    }

    #[test]
    fn test_parse_dev_diagnostic_mode() {
        let yaml = "dev:\n  diagnostic_mode: true\n";
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(config.dev.diagnostic_mode);
    }

    #[test]
    fn test_split_ratio_default() {
        let ratio = SplitRatioConfig::default();
        assert!((ratio.agents - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validate_rejects_unknown_top_level_field() {
        let yaml = "main_branch: develop\nunknown_key: true\n";
        let err = serde_yml::from_str::<Config>(yaml).unwrap_err().to_string();
        assert!(err.contains("unknown field"));
        assert!(err.contains("unknown_key"));
    }

    #[test]
    fn test_validate_rejects_unknown_nested_field() {
        let yaml = "dev:\n  diagnostic_mode: true\n  extra: true\n";
        let err = serde_yml::from_str::<Config>(yaml).unwrap_err().to_string();
        assert!(err.contains("unknown field"));
        assert!(err.contains("extra"));
    }

    #[test]
    fn test_validate_rejects_invalid_split_ratio() {
        let mut config = Config::default();
        config.split_ratio.agents = 1.0;
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("split_ratio.agents"));
    }

    #[test]
    fn test_validate_rejects_duplicate_group_agent() {
        let mut config = Config::default();
        config.group.push("claude".into());
        let err = config.validate().unwrap_err().to_string();
        assert!(err.contains("duplicate agent"));
    }
}
