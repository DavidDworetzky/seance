pub mod branch;
pub mod merge;
pub mod worktree;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn repo_root(path: &Path) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .with_context(|| format!("detecting git repo root from {}", path.display()))?;

    if output.status.success() {
        let root = PathBuf::from(String::from_utf8(output.stdout)?.trim());
        return Ok(normalize_path(&root)?);
    }

    normalize_path(path)
}

pub fn primary_repo_root(path: &Path) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(path)
        .output()
        .with_context(|| format!("detecting git common dir from {}", path.display()))?;

    if output.status.success() {
        let common_dir = PathBuf::from(String::from_utf8(output.stdout)?.trim());
        if common_dir.file_name().and_then(|name| name.to_str()) == Some(".git") {
            if let Some(repo_root) = common_dir.parent() {
                return normalize_path(repo_root);
            }
        }
    }

    repo_root(path)
}

fn normalize_path(path: &Path) -> Result<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Ok(path);
    }

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(std::env::current_dir()?.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_repo_root_falls_back_for_non_repo_path() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            primary_repo_root(dir.path()).unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }
}
