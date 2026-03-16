use crate::utils::{Config, copy_dir_all, images2webp, load_page};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;
use tera::{Context as TeraContext, Tera};

pub fn execute(config_path: &str) -> Result<()> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    let output_dir = Path::new(&config.output_dir);
    let content_dir = Path::new("content");
    let theme_dir = Path::new("theme");
    println!(
        "{} {}",
        "Building site to".cyan(),
        output_dir.display().to_string().cyan()
    );
    let tera = Tera::new("theme/**/*.html")?;
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    let assets_src = content_dir.join("assets");
    if assets_src.exists() {
        copy_dir_all(&assets_src, output_dir.join("assets"))?;
    }

    let images_src = output_dir.join("assets");
    let images_dst = output_dir.join("assets");
    images2webp(&images_src, &images_dst)?;

    let fonts_src = theme_dir.join("fonts");
    if fonts_src.exists() {
        copy_dir_all(&fonts_src, output_dir.join("fonts"))?;
    }

    let favicon_src = theme_dir.join("favicon.ico");
    if favicon_src.exists() {
        fs::copy(&favicon_src, output_dir.join("favicon.ico"))?;
    }

    for item in &config.nav {
        match item.item_type.as_str() {
            "page" => {
                let page = load_page(content_dir, &item.path, false)?;
                let mut ctx = TeraContext::new();
                ctx.insert("config", &config);
                ctx.insert("current_page", &page);
                ctx.insert("content", &page.content_html);
                ctx.insert("root_path", ".");

                let render_out = tera.render("page.html", &ctx)?;
                fs::write(output_dir.join(&page.url), render_out)?;
            }
            "link" => {
                // link 类型指向外部网站，无需生成页面
            }
            _ => {}
        }
    }

    println!("{}", "Build success!".green().bold());
    Ok(())
}
