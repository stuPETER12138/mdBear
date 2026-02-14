use crate::cmd::build;
use crate::utils::Config;
use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::sync::mpsc;
use std::time::Duration;

pub async fn execute(port: u16, config_path: &str) -> Result<()> {
    println!("Building...");
    build::execute(config_path)?;

    let config_str = fs::read_to_string(config_path).context("Failed to read config file")?;
    let config: Config = toml::from_str(&config_str).context("Failed to parse config")?;
    let output_dir = config.output_dir.clone();
    let url = format!("http://localhost:{}", port);

    let url_clone = url.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        match webbrowser::open(&url_clone) {
            Ok(_) => println!("✓ Browser opened: {}", url_clone),
            Err(e) => eprintln!("⚠ Failed to open browser: {}", e),
        }
    });

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default())?;

    if std::path::Path::new("content").exists() {
        watcher.watch(std::path::Path::new("content"), RecursiveMode::Recursive)?;
        println!("✓ Watching content directory for changes");
    }

    if std::path::Path::new("theme").exists() {
        watcher.watch(std::path::Path::new("theme"), RecursiveMode::Recursive)?;
        println!("✓ Watching theme directory for changes");
    }

    if std::path::Path::new(config_path).exists() {
        watcher.watch(
            std::path::Path::new(config_path),
            RecursiveMode::NonRecursive,
        )?;
        println!("✓ Watching config file for changes");
    }

    let config_path_for_thread = config_path.to_string();
    tokio::spawn(async move {
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    Ok(_) => {
                        println!("\n🔄 Detected file change, rebuilding...");
                        if let Err(e) = build::execute(&config_path_for_thread) {
                            eprintln!("⚠️  Build failed: {}", e);
                        } else {
                            println!("✓ Rebuild completed successfully");
                        }
                    }
                    Err(e) => eprintln!("⚠️  Watch error: {}", e),
                },
                Err(e) => {
                    eprintln!("⚠️  Channel error: {}", e);
                    break;
                }
            }
        }
    });

    println!("Watching for changes in content/, theme/, and config.toml...");

    let routes = warp::fs::dir(output_dir);
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;

    println!("Press Ctrl+C to stop");

    Ok(())
}
