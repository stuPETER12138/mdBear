use crate::utils::{Config, copy_dir_all, load_page};
use anyhow::Result;
use std::fs;
use std::path::Path;
use tera::{Context as TeraContext, Tera};

pub fn execute(config_path: &str) -> Result<()> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    let output_dir = Path::new(&config.output_dir);
    let content_dir = Path::new("content");
    let theme_dir = Path::new("theme");
    println!("Building site to {:?}", output_dir);
    let tera = Tera::new("theme/**/*.html")?;
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    let assets_src = content_dir.join("assets");
    if assets_src.exists() {
        copy_dir_all(&assets_src, output_dir.join("assets"))?;
    }
    let fonts_src = theme_dir.join("fonts");
    if fonts_src.exists() {
        copy_dir_all(&fonts_src, output_dir.join("fonts"))?;
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
            "blog" => {
                let section_path = content_dir.join(&item.path);
                let section_out_dir = output_dir.join(&item.path);
                fs::create_dir_all(&section_out_dir)?;

                let mut posts = Vec::new();
                if section_path.exists() {
                    for entry in fs::read_dir(&section_path)? {
                        let entry = entry?;
                        if entry.path().extension().map_or(false, |e| e == "md") {
                            let rel_path =
                                format!("{}/{}", item.path, entry.file_name().to_str().unwrap());
                            if let Ok(page) = load_page(content_dir, &rel_path, true) {
                                let mut ctx = TeraContext::new();
                                ctx.insert("config", &config);
                                ctx.insert("current_page", &page);
                                ctx.insert("content", &page.content_html);
                                ctx.insert("root_path", "..");

                                let out_path = section_out_dir.join(&page.url);
                                fs::write(out_path, tera.render("page.html", &ctx)?)?;
                                posts.push(page);
                            }
                        }
                    }
                }
                posts.sort_by(|a, b| b.meta.date.cmp(&a.meta.date));

                let mut ctx = TeraContext::new();
                ctx.insert("config", &config);
                ctx.insert("section_title", &item.name);
                ctx.insert("posts", &posts);
                ctx.insert("root_path", "..");

                fs::write(
                    section_out_dir.join("index.html"),
                    tera.render("list.html", &ctx)?,
                )?;
            }
            _ => {}
        }
    }

    println!("Build success!");
    Ok(())
}
