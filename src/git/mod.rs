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

fn normalize_path(path: &Path) -> Result<PathBuf> {
    if let Ok(path) = path.canonicalize() {
        return Ok(path);
    }

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(std::env::current_dir()?.join(path))
}
