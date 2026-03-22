use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitArgs {
    /// Path to initialize (defaults to current directory)
    pub path: Option<String>,

    /// Also install Ghostty keybindings
    #[arg(long)]
    pub keybindings: bool,
}

pub async fn run(args: InitArgs) -> Result<()> {
    let path = args.path.map(PathBuf::from).unwrap_or(std::env::current_dir()?);
    let config_path = path.join(".seance.yaml");

    if config_path.exists() {
        anyhow::bail!(".seance.yaml already exists at {}", path.display());
    }

    // Detect project
    let project_type = if path.join("Cargo.toml").exists() {
        "Rust"
    } else if path.join("package.json").exists() {
        "Node"
    } else if path.join("go.mod").exists() {
        "Go"
    } else {
        "Unknown"
    };

    println!("Seance — initializing project\n");
    println!("Detected: {} project", project_type);

    // Detect main branch
    let main_branch = detect_main_branch(&path);
    println!("Main branch: {}\n", main_branch);

    // Generate default config
    let default_config = generate_default_config(&main_branch, project_type);
    std::fs::write(&config_path, &default_config)?;
    println!("Written: {}", config_path.display());

    if args.keybindings {
        install_keybindings()?;
    }

    Ok(())
}

fn detect_main_branch(path: &PathBuf) -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let branch = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // Check if main or master exists
            for candidate in &["main", "master"] {
                let check = std::process::Command::new("git")
                    .args(["rev-parse", "--verify", candidate])
                    .current_dir(path)
                    .output();
                if let Ok(c) = check {
                    if c.status.success() {
                        return candidate.to_string();
                    }
                }
            }
            branch
        }
        _ => "main".into(),
    }
}

fn generate_default_config(main_branch: &str, project_type: &str) -> String {
    let mut config = format!(
        r#"main_branch: {}
worktree_dir: "../{{project}}__seance"

# Agents per worktree
group:
  - claude
  - codex

merge_strategy: squash
"#,
        main_branch
    );

    // Add project-specific hooks and file ops
    match project_type {
        "Rust" => {
            config.push_str(
                r#"
post_create:
  - cargo build

pre_merge:
  - cargo test
  - cargo clippy -- -D warnings

files:
  symlink:
    - target
"#,
            );
        }
        "Node" => {
            config.push_str(
                r#"
post_create:
  - npm install

pre_merge:
  - npm test

files:
  copy:
    - .env
  symlink:
    - node_modules
"#,
            );
        }
        _ => {}
    }

    config
}

fn install_keybindings() -> Result<()> {
    let ghostty_config = dirs::config_dir()
        .unwrap_or_default()
        .join("ghostty")
        .join("config");

    let keybindings = r#"
# Seance keybindings
keybind = ctrl+s>d=text:seance\n
keybind = ctrl+s>a=text:seance add\n
keybind = ctrl+s>c=text:seance add --circle\n
keybind = ctrl+s>x=text:seance remove\n
keybind = ctrl+s>m=text:seance merge\n
keybind = ctrl+s>z=text:seance sleep\n
keybind = ctrl+s>w=text:seance wake\n
keybind = ctrl+s>l=text:seance list\n
keybind = ctrl+s>1=text:seance focus 1\n
keybind = ctrl+s>2=text:seance focus 2\n
keybind = ctrl+s>3=text:seance focus 3\n
keybind = ctrl+s>4=text:seance focus 4\n
keybind = ctrl+s>5=text:seance focus 5\n
keybind = ctrl+s>6=text:seance focus 6\n
keybind = ctrl+s>7=text:seance focus 7\n
keybind = ctrl+s>8=text:seance focus 8\n
keybind = ctrl+s>n=text:seance focus --next\n
keybind = ctrl+s>p=text:seance focus --prev\n
"#;

    if ghostty_config.exists() {
        let mut contents = std::fs::read_to_string(&ghostty_config)?;
        if contents.contains("# Seance keybindings") {
            println!("Keybindings already installed.");
            return Ok(());
        }
        contents.push_str(keybindings);
        std::fs::write(&ghostty_config, contents)?;
    } else {
        std::fs::create_dir_all(ghostty_config.parent().unwrap())?;
        std::fs::write(&ghostty_config, keybindings)?;
    }

    println!("Installed keybindings to {}", ghostty_config.display());
    println!("Reload Ghostty config to activate.");
    Ok(())
}
