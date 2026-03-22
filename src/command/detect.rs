use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct DetectArgs {
    /// Additional paths to scan
    pub paths: Vec<String>,
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
    let home = dirs::home_dir().unwrap_or_default();
    let mut scan_dirs: Vec<PathBuf> = DEFAULT_SCAN_DIRS
        .iter()
        .map(|d| {
            let expanded = d.replace('~', &home.to_string_lossy());
            PathBuf::from(expanded)
        })
        .collect();

    for p in &args.paths {
        scan_dirs.push(PathBuf::from(p));
    }

    // Check custom scan paths
    if let Some(config_dir) = dirs::config_dir() {
        let custom_paths = config_dir.join("seance").join("scan_paths");
        if custom_paths.exists() {
            let contents = std::fs::read_to_string(&custom_paths)?;
            for line in contents.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    scan_dirs.push(PathBuf::from(trimmed));
                }
            }
        }
    }

    println!("{:<50} {:<15} {:<10}", "Repository", "Type", "Config");
    println!("{}", "-".repeat(75));

    for dir in &scan_dirs {
        if !dir.exists() {
            continue;
        }
        scan_directory(dir, 1)?;
    }

    Ok(())
}

fn scan_directory(dir: &PathBuf, depth: usize) -> Result<()> {
    if depth > 3 {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let git_dir = path.join(".git");
        if git_dir.exists() {
            let project_type = detect_project_type(&path);
            let has_config = path.join(".seance.yaml").exists();

            println!(
                "{:<50} {:<15} {:<10}",
                path.display(),
                project_type,
                if has_config { "✓" } else { "--" }
            );
        } else {
            scan_directory(&path, depth + 1)?;
        }
    }

    Ok(())
}

fn detect_project_type(path: &PathBuf) -> &'static str {
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
