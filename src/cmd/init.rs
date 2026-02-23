use crate::utils::DefaultAssets;
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn execute(name: &str) -> Result<()> {
    let root = Path::new(name);

    if root.exists() {
        println!(
            "{} {}",
            "Initializing project in existing directory:".yellow(),
            name.cyan()
        );
    } else {
        println!("{} {}", "Initializing project:".yellow(), name.cyan());
        fs::create_dir(root)?;
    }

    // Get top-level items from defaults and sync them
    let items: Vec<_> = DefaultAssets::iter()
        .filter_map(|f| {
            let path = Path::new(f.as_ref());
            path.components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .map(String::from)
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for item in &items {
        let target = root.join(item);
        if target.exists() {
            println!(
                "  {} {}",
                "Skipping (exists):".bright_black(),
                item.bright_black()
            );
            continue;
        }

        println!("  {} {}", "Creating:".green(), item.green());
        for filename in DefaultAssets::iter() {
            let file_path = Path::new(filename.as_ref());
            let first = file_path
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str());
            if first == Some(item.as_str()) {
                let target_path = root.join(file_path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let file = DefaultAssets::get(filename.as_ref()).unwrap();
                fs::write(target_path, file.data.as_ref())?;
            }
        }
    }

    // Initialize git repository
    if root.join(".git").exists() {
        println!(
            "  {} {}",
            "Git repository already exists, skipping:".bright_black(),
            "skipped".bright_black()
        );
    } else {
        println!("{}", "Initializing git repository...".blue());
        Command::new("git").arg("init").current_dir(root).output()?;
    }

    println!(
        "\n{}\nPlease run:\n  {} {}\n  {}",
        "Project initialized!".green().bold(),
        "cd".cyan(),
        name.cyan(),
        "mdbear build".cyan()
    );

    Ok(())
}
