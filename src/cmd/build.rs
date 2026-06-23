use crate::utils::{Config, copy_dir_all, images2webp, load_page, scan_blog_posts};
use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;
use tera::{Context as TeraContext, Result as TeraResult, Tera, Value};

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
    let mut tera = Tera::new("theme/**/*.html")?;

    // Register custom filters
    tera.register_filter("truncate", truncate_filter);
    tera.register_filter("date_format", date_format_filter);
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

    let style_src = theme_dir.join("style.css");
    if style_src.exists() {
        fs::copy(&style_src, output_dir.join("style.css"))?;
    }

    for item in &config.nav {
        match item.item_type.as_str() {
            "page" => {
                if !item.path.ends_with(".md") {
                    continue;
                }
                let full_path = content_dir.join(&item.path);
                if !full_path.exists() {
                    continue;
                }
                let page = load_page(content_dir, &item.path, false)?;
                let mut ctx = TeraContext::new();
                ctx.insert("config", &config);
                ctx.insert("current_page", &page);
                ctx.insert("content", &page.content_html);
                ctx.insert("root_path", ".");

                let render_out = tera.render("page.html", &ctx)?;
                fs::write(output_dir.join(&page.url), render_out)?;
            }
            "link" => {}
            _ => {}
        }
    }

    // Generate blog pages from content/blog/
    let blog_posts = scan_blog_posts(content_dir)?;
    if !blog_posts.is_empty() {
        println!(
            "{} {} {}",
            "发现".cyan(),
            blog_posts.len().to_string().cyan(),
            "篇博客".cyan()
        );

        // Render individual blog post pages
        for post in &blog_posts {
            let mut ctx = TeraContext::new();
            ctx.insert("config", &config);
            ctx.insert("current_page", &post);
            ctx.insert("content", &post.content_html);
            ctx.insert("root_path", "..");

            let render_out = tera.render("post.html", &ctx)?;
            let post_path = output_dir.join(&post.url);
            if let Some(parent) = post_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&post_path, render_out)?;
            println!(
                "  {} {}",
                "生成:".green(),
                post_path.display().to_string().green()
            );
        }

        // Render blog listing page
        let mut ctx = TeraContext::new();
        ctx.insert("config", &config);
        ctx.insert("posts", &blog_posts);
        ctx.insert("root_path", ".");

        let render_out = tera.render("blog.html", &ctx)?;
        let blog_path = output_dir.join("blog.html");
        fs::write(&blog_path, render_out)?;
        println!(
            "  {} {}",
            "生成博客列表:".green(),
            blog_path.display().to_string().green()
        );
    }

    println!("{}", "Build success!".green().bold());
    Ok(())
}

fn truncate_filter(
    value: &Value,
    args: &std::collections::HashMap<String, Value>,
) -> TeraResult<Value> {
    let s = value.as_str().unwrap_or("");
    let length = args.get("length").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let ellipsis = args
        .get("ellipsis")
        .and_then(|v| v.as_str())
        .unwrap_or("...");

    if s.len() <= length {
        return Ok(Value::String(s.to_string()));
    }

    let truncated = s.chars().take(length).collect::<String>();
    Ok(Value::String(format!("{}{}", truncated, ellipsis)))
}

fn date_format_filter(
    value: &Value,
    args: &std::collections::HashMap<String, Value>,
) -> TeraResult<Value> {
    use chrono::{DateTime, NaiveDate};

    let date_str = value.as_str().unwrap_or("");
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%Y-%m-%d");

    // Try parsing as ISO date (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Ok(Value::String(date.format(format).to_string()));
    }

    // Try parsing as RFC3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(Value::String(dt.format(format).to_string()));
    }

    // Return original if parsing fails
    Ok(Value::String(date_str.to_string()))
}
