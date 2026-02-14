use crate::utils::DefaultAssets;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn execute(name: &str) -> Result<()> {
    let root = Path::new(name);
    if root.exists() {
        println!("Error: Directory '{}' already exists.", name);
        return Ok(());
    }

    println!("Initializing project: {} ...", name);
    fs::create_dir(root)?;

    for filename in DefaultAssets::iter() {
        let file_path = filename.as_ref();
        let embedded_file = DefaultAssets::get(file_path).unwrap();
        let target_path = root.join(file_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        println!("  Creating: {}", file_path);
        fs::write(target_path, embedded_file.data.as_ref())?;
    }

    println!(
        "Project initialized!\nPlease run:\n  cd {}\n  mdbear build",
        name
    );

    Ok(())
}
