use anyhow::{Context, Result};
use clap::Args;
use std::io::{self, Write};

#[derive(Args)]
pub struct RepoArgs {
    /// Additional paths to scan
    pub paths: Vec<String>,
}

pub async fn run(args: RepoArgs) -> Result<()> {
    let mut repos = crate::command::detect::discover_repositories(&args.paths)?;
    repos.sort_by(|left, right| {
        right
            .has_config
            .cmp(&left.has_config)
            .then_with(|| left.path.cmp(&right.path))
    });

    if repos.is_empty() {
        println!("No repos found in the autodetected scan paths.");
        return Ok(());
    }

    println!("Detected repositories:\n");
    for (index, repo) in repos.iter().enumerate() {
        let config = if repo.has_config { "config" } else { "default" };
        println!(
            "  [{}] {}  ({}, {})",
            index + 1,
            repo.path.display(),
            repo.project_type,
            config
        );
    }
    println!();

    let selection = prompt_for_selection(repos.len())?;
    let Some(index) = selection else {
        println!("Cancelled.");
        return Ok(());
    };

    let repo = &repos[index];
    println!("Opening Seance in {}", repo.path.display());
    std::env::set_current_dir(&repo.path)
        .with_context(|| format!("changing directory to {}", repo.path.display()))?;
    crate::dashboard::run_default().await
}

fn prompt_for_selection(len: usize) -> Result<Option<usize>> {
    print!("Select a repo [1-{}, Enter to cancel]: ", len);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    parse_selection(&input, len)
}

fn parse_selection(input: &str, len: usize) -> Result<Option<usize>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let pick = trimmed
        .parse::<usize>()
        .with_context(|| format!("invalid selection: {}", trimmed))?;
    if !(1..=len).contains(&pick) {
        anyhow::bail!("selection must be between 1 and {}", len);
    }

    Ok(Some(pick - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selection_accepts_empty_input() {
        assert_eq!(parse_selection("", 3).unwrap(), None);
        assert_eq!(parse_selection("   ", 3).unwrap(), None);
    }

    #[test]
    fn test_parse_selection_accepts_valid_index() {
        assert_eq!(parse_selection("2", 3).unwrap(), Some(1));
    }

    #[test]
    fn test_parse_selection_rejects_invalid_input() {
        assert!(parse_selection("abc", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
    }
}
