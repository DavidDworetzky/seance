pub mod schema;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use schema::Config;

const PROJECT_CONFIG_NAME: &str = ".seance.yaml";
const GLOBAL_CONFIG_DIR: &str = "seance";

impl Config {
    /// Load config by searching up from `start_dir` for .seance.yaml,
    /// then falling back to ~/.config/seance/config.yaml.
    /// Missing config files are not an error — returns defaults.
    pub fn load(start_dir: Option<&Path>) -> Result<Self> {
        let project_config = start_dir
            .map(|d| find_config_upward(d))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|d| find_config_upward(&d))
            });

        let global_config = global_config_path();

        let mut config = Config::default();

        // Load global first (lower priority)
        if let Some(path) = &global_config {
            if path.exists() {
                let contents = std::fs::read_to_string(path)
                    .with_context(|| format!("reading global config: {}", path.display()))?;
                config = serde_yaml::from_str(&contents)
                    .with_context(|| format!("parsing global config: {}", path.display()))?;
            }
        }

        // Override with project config (higher priority)
        if let Some(path) = &project_config {
            if path.exists() {
                let contents = std::fs::read_to_string(path)
                    .with_context(|| format!("reading project config: {}", path.display()))?;
                config = serde_yaml::from_str(&contents)
                    .with_context(|| format!("parsing project config: {}", path.display()))?;
            }
        }

        Ok(config)
    }
}

fn find_config_upward(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(PROJECT_CONFIG_NAME);
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(GLOBAL_CONFIG_DIR).join("config.yaml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_config_upward_found() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("a/b/c");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(dir.path().join(".seance.yaml"), "main_branch: test\n").unwrap();

        let found = find_config_upward(&sub);
        assert!(found.is_some());
        assert_eq!(found.unwrap(), dir.path().join(".seance.yaml"));
    }

    #[test]
    fn test_find_config_upward_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let found = find_config_upward(dir.path());
        assert!(found.is_none());
    }

    #[test]
    fn test_load_from_project_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".seance.yaml"),
            "main_branch: develop\nmerge_strategy: rebase\n",
        )
        .unwrap();

        let config = Config::load(Some(dir.path())).unwrap();
        assert_eq!(config.main_branch, "develop");
    }

    #[test]
    fn test_load_defaults_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(Some(dir.path())).unwrap();
        assert_eq!(config.main_branch, "main");
        assert_eq!(config.group, vec!["claude", "codex"]);
    }
}
