use anyhow::{Context, Result};
use std::path::Path;

use crate::config::schema::Config;

/// Run file copy and symlink operations from main worktree into new worktree.
pub fn run_file_ops(config: &Config, worktree_path: &Path) -> Result<()> {
    let main_worktree = std::env::current_dir()?;

    // Copy files
    for pattern in &config.files.copy {
        let matches = glob::glob(&main_worktree.join(pattern).to_string_lossy())
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?;

        for entry in matches {
            let src = entry?;
            let relative = src.strip_prefix(&main_worktree)?;
            let dst = worktree_path.join(relative);

            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if src.is_dir() {
                copy_dir_recursive(&src, &dst)?;
            } else {
                std::fs::copy(&src, &dst)?;
            }

            tracing::debug!("Copied: {} -> {}", src.display(), dst.display());
        }
    }

    // Symlink files
    for pattern in &config.files.symlink {
        let src = main_worktree.join(pattern);
        if !src.exists() {
            continue;
        }

        let dst = worktree_path.join(pattern);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Remove existing target if it exists
        if dst.exists() || dst.symlink_metadata().is_ok() {
            if dst.is_dir() {
                std::fs::remove_dir_all(&dst)?;
            } else {
                std::fs::remove_file(&dst)?;
            }
        }

        std::os::unix::fs::symlink(&src, &dst)
            .with_context(|| format!("Failed to symlink {} -> {}", src.display(), dst.display()))?;

        tracing::debug!("Symlinked: {} -> {}", dst.display(), src.display());
    }

    Ok(())
}

/// Run a shell hook command in the worktree directory.
pub fn run_hook(command: &str, cwd: &Path) -> Result<()> {
    let status = std::process::Command::new("sh")
        .args(["-c", command])
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to run hook: {}", command))?;

    if !status.success() {
        anyhow::bail!(
            "Hook failed (exit {}): {}",
            status.code().unwrap_or(-1),
            command
        );
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{Config, FileConfig};

    #[test]
    fn test_run_hook_success() {
        let dir = tempfile::tempdir().unwrap();
        run_hook("echo hello", dir.path()).unwrap();
    }

    #[test]
    fn test_run_hook_failure() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_hook("exit 1", dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_copy_file_ops() {
        let main_dir = tempfile::tempdir().unwrap();
        let wt_dir = tempfile::tempdir().unwrap();

        // Create a file in "main worktree"
        std::fs::write(main_dir.path().join(".env"), "SECRET=123").unwrap();

        let mut config = Config::default();
        config.files = FileConfig {
            copy: vec![".env".into()],
            symlink: vec![],
        };

        // We need to be in the main dir for this to work
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(main_dir.path()).unwrap();

        run_file_ops(&config, wt_dir.path()).unwrap();

        std::env::set_current_dir(original_dir).unwrap();

        // Verify copy
        let copied = std::fs::read_to_string(wt_dir.path().join(".env")).unwrap();
        assert_eq!(copied, "SECRET=123");
    }

    #[test]
    fn test_symlink_file_ops() {
        let main_dir = tempfile::tempdir().unwrap();
        let wt_dir = tempfile::tempdir().unwrap();

        // Create a directory in "main worktree"
        let node_modules = main_dir.path().join("node_modules");
        std::fs::create_dir(&node_modules).unwrap();
        std::fs::write(node_modules.join("test.js"), "// test").unwrap();

        let mut config = Config::default();
        config.files = FileConfig {
            copy: vec![],
            symlink: vec!["node_modules".into()],
        };

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(main_dir.path()).unwrap();

        run_file_ops(&config, wt_dir.path()).unwrap();

        std::env::set_current_dir(original_dir).unwrap();

        // Verify symlink
        let link = wt_dir.path().join("node_modules");
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    }

    #[test]
    fn test_copy_dir_recursive() {
        let src_dir = tempfile::tempdir().unwrap();
        let dst_dir = tempfile::tempdir().unwrap();
        let dst = dst_dir.path().join("copied");

        std::fs::write(src_dir.path().join("a.txt"), "aaa").unwrap();
        std::fs::create_dir(src_dir.path().join("sub")).unwrap();
        std::fs::write(src_dir.path().join("sub/b.txt"), "bbb").unwrap();

        copy_dir_recursive(src_dir.path(), &dst).unwrap();

        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "aaa");
        assert_eq!(
            std::fs::read_to_string(dst.join("sub/b.txt")).unwrap(),
            "bbb"
        );
    }
}
