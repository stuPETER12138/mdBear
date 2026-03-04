use crate::utils::DefaultAssets;
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub fn execute(project_path: &str) -> Result<()> {
    let project_dir = PathBuf::from(project_path);

    if !project_dir.exists() {
        anyhow::bail!("Project directory '{}' does not exist", project_path);
    }

    let theme_dir = project_dir.join("theme");
    if !theme_dir.exists() {
        anyhow::bail!(
            "Theme directory '{}' not found. Please run 'mdbear init' first or ensure the theme directory exists.",
            theme_dir.display()
        );
    }

    println!(
        "{} {}",
        "Syncing theme files to:".cyan(),
        theme_dir.display().to_string().cyan()
    );

    let mut synced_count = 0;
    let mut updated_count = 0;

    for filename in DefaultAssets::iter() {
        let file_path = filename.as_ref();
        // Only sync theme files
        if !file_path.starts_with("theme/") {
            continue;
        }
        let embedded_file = DefaultAssets::get(file_path).unwrap();
        let target_path = theme_dir.join(file_path.strip_prefix("theme/").unwrap());

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let needs_update = if target_path.exists() {
            let existing_content = fs::read(&target_path)?;
            existing_content != embedded_file.data.as_ref()
        } else {
            true
        };

        if needs_update {
            println!(
                "  {} {}",
                "Updating:".yellow(),
                file_path.strip_prefix("theme/").unwrap().yellow()
            );
            fs::write(&target_path, embedded_file.data.as_ref())?;
            updated_count += 1;
        } else {
            println!(
                "  {} {}",
                "Unchanged:".bright_black(),
                file_path.strip_prefix("theme/").unwrap().bright_black()
            );
        }
        synced_count += 1;
    }

    println!(
        "\n{} {} {}, {} {}",
        "Sync complete!".green().bold(),
        synced_count.to_string().green(),
        "theme files checked".green(),
        updated_count.to_string().yellow(),
        "updated".yellow()
    );

    Ok(())
}
