use anyhow::Result;
use clap::Args;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct DetectArgs {
    /// Additional paths to scan
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedRepo {
    pub path: PathBuf,
    pub project_type: &'static str,
    pub has_config: bool,
}

const DEFAULT_SCAN_DIRS: &[&str] = &[
    "~/Documents/repos",
    "~/Developer",
    "~/Projects",
    "~/Code",
    "~/src",
    "~/workspace",
];

pub async fn run(args: DetectArgs) -> Result<()> {
    let repos = discover_repositories(&args.paths)?;

    println!("{:<50} {:<15} {:<10}", "Repository", "Type", "Config");
    println!("{}", "-".repeat(75));

    for repo in repos {
        println!(
            "{:<50} {:<15} {:<10}",
            repo.path.display(),
            repo.project_type,
            if repo.has_config { "✓" } else { "--" }
        );
    }

    Ok(())
}

pub fn discover_repositories(extra_paths: &[String]) -> Result<Vec<DetectedRepo>> {
    let scan_dirs = scan_directories(extra_paths)?;
    discover_repositories_in_dirs(&scan_dirs)
}

fn discover_repositories_in_dirs(scan_dirs: &[PathBuf]) -> Result<Vec<DetectedRepo>> {
    let mut repos = BTreeMap::new();

    for dir in scan_dirs {
        if !dir.exists() {
            continue;
        }
        scan_directory(dir, 1, &mut repos)?;
    }

    Ok(repos.into_values().collect())
}

fn scan_directories(extra_paths: &[String]) -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut scan_dirs: Vec<PathBuf> = DEFAULT_SCAN_DIRS
        .iter()
        .map(|d| expand_home_dir(d, &home))
        .collect();

    for path in extra_paths {
        scan_dirs.push(PathBuf::from(path));
    }

    if let Some(config_dir) = dirs::config_dir() {
        let custom_paths = config_dir.join("seance").join("scan_paths");
        if custom_paths.exists() {
            let contents = std::fs::read_to_string(&custom_paths)?;
            for line in contents.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    scan_dirs.push(expand_home_dir(trimmed, &home));
                }
            }
        }
    }

    Ok(scan_dirs)
}

fn expand_home_dir(path: &str, home: &Path) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        return home.join(stripped);
    }
    if path == "~" {
        return home.to_path_buf();
    }

    PathBuf::from(path)
}

fn scan_directory(
    dir: &Path,
    depth: usize,
    repos: &mut BTreeMap<String, DetectedRepo>,
) -> Result<()> {
    if depth > 3 {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        if path.join(".git").exists() {
            let canonical = path.canonicalize().unwrap_or(path);
            let key = canonical.to_string_lossy().to_string();
            repos.entry(key).or_insert_with(|| DetectedRepo {
                project_type: detect_project_type(&canonical),
                has_config: canonical.join(".seance.yaml").exists(),
                path: canonical,
            });
        } else {
            scan_directory(&path, depth + 1, repos)?;
        }
    }

    Ok(())
}

fn detect_project_type(path: &Path) -> &'static str {
    if path.join("Cargo.toml").exists() {
        "Rust"
    } else if path.join("package.json").exists() {
        "Node"
    } else if path.join("go.mod").exists() {
        "Go"
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        "Python"
    } else if path.join("Gemfile").exists() {
        "Ruby"
    } else if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
        "Java"
    } else {
        "Unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home_dir() {
        let home = PathBuf::from("/tmp/home");
        assert_eq!(expand_home_dir("~/repos", &home), home.join("repos"));
        assert_eq!(expand_home_dir("~", &home), home);
        assert_eq!(
            expand_home_dir("/tmp/elsewhere", &home),
            PathBuf::from("/tmp/elsewhere")
        );
    }

    #[test]
    fn test_discover_repositories_finds_git_repo() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("sample");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = 'sample'\n").unwrap();
        std::fs::write(repo.join(".seance.yaml"), "main_branch: main\n").unwrap();

        let repos = discover_repositories_in_dirs(&[root.path().to_path_buf()]).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].path, repo.canonicalize().unwrap());
        assert_eq!(repos[0].project_type, "Rust");
        assert!(repos[0].has_config);
    }
}
