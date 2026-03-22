use anyhow::Result;
use std::path::Path;

/// Write a prompt file to the worktree's .seance/ directory.
pub fn write_prompt_file(worktree_path: &Path, branch: &str, content: &str) -> Result<String> {
    let seance_dir = worktree_path.join(".seance");
    std::fs::create_dir_all(&seance_dir)?;

    let sanitized = branch.replace('/', "-");
    let filename = format!("PROMPT-{}.md", sanitized);
    let path = seance_dir.join(&filename);

    std::fs::write(&path, content)?;

    Ok(path.to_string_lossy().to_string())
}

/// Read a prompt from a file path or inline text.
pub fn resolve_prompt(inline: Option<&str>, file: Option<&str>) -> Result<Option<String>> {
    if let Some(text) = inline {
        return Ok(Some(text.to_string()));
    }
    if let Some(path) = file {
        let content = std::fs::read_to_string(path)?;
        return Ok(Some(content));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_write_prompt_file() {
        let dir = tempfile::tempdir().unwrap();
        let wt_path = dir.path();
        let path = write_prompt_file(wt_path, "feat/auth", "Fix the auth bug").unwrap();
        assert!(PathBuf::from(&path).exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Fix the auth bug");
    }

    #[test]
    fn test_write_prompt_file_sanitizes_branch() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_prompt_file(dir.path(), "feat/some/nested", "hello").unwrap();
        assert!(path.contains("PROMPT-feat-some-nested.md"));
    }

    #[test]
    fn test_resolve_prompt_inline() {
        let result = resolve_prompt(Some("do the thing"), None).unwrap();
        assert_eq!(result, Some("do the thing".into()));
    }

    #[test]
    fn test_resolve_prompt_none() {
        let result = resolve_prompt(None, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_prompt_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("prompt.md");
        std::fs::write(&file, "from file").unwrap();
        let result = resolve_prompt(None, Some(file.to_str().unwrap())).unwrap();
        assert_eq!(result, Some("from file".into()));
    }
}
