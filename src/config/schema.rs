use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
                        command: "claude".into(),
                        prompt_injection: PromptInjection::Trailing,
                        auto_name: true,
                        auto_name_command: None,
                    },
                ),
                (
                    "codex".into(),
                    AgentConfig {
                        command: "codex".into(),
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
            theme: "dark".into(),
            status_icons: StatusIcons::default(),
            monitors: MonitorConfig::default(),
            pr: PrConfig::default(),
            auto_name: AutoNameConfig::default(),
            quadrants_per_monitor: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
pub struct FileConfig {
    pub copy: Vec<String>,
    pub symlink: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.group, vec!["claude", "codex"]);
        assert_eq!(config.quadrants_per_monitor, 4);
        assert!(config.agents.contains_key("claude"));
        assert!(config.agents.contains_key("codex"));
    }

    #[test]
    fn test_default_agents_config() {
        let config = Config::default();
        let claude = config.agents.get("claude").unwrap();
        assert_eq!(claude.command, "claude");
        assert!(claude.auto_name);

        let codex = config.agents.get("codex").unwrap();
        assert_eq!(codex.command, "codex");
        assert!(!codex.auto_name);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.main_branch, config.main_branch);
        assert_eq!(parsed.group, config.group);
    }

    #[test]
    fn test_parse_minimal_yaml() {
        let yaml = "main_branch: develop\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
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
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.group, vec!["claude", "codex", "gemini"]);
    }

    #[test]
    fn test_merge_strategy_serde() {
        let yaml = "merge_strategy: rebase\n";
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(config.merge_strategy, MergeStrategy::Rebase));
    }

    #[test]
    fn test_status_icons_defaults() {
        let icons = StatusIcons::default();
        assert_eq!(icons.working, "⚡");
        assert_eq!(icons.done, "✓");
    }

    #[test]
    fn test_split_ratio_default() {
        let ratio = SplitRatioConfig::default();
        assert!((ratio.agents - 0.8).abs() < f64::EPSILON);
    }
}
