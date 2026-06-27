use crate::utils::{Config, copy_dir_all, generate_rss, images2webp, load_page, scan_blog_posts};
use anyhow::{Context, Result, bail};
use colored::Colorize;
use serde::{Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Result as TeraResult, Tera, Value};

pub fn execute(config_path: &str) -> Result<()> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    let output_dir = Path::new(&config.output_dir);
    let content_dir = Path::new("content");
    let theme_dir = Path::new("theme");
    validate_output_dir(output_dir, content_dir, theme_dir)?;
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
    let converted_images = images2webp(&images_src, &images_dst)?;

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
                let page = load_page(content_dir, &item.path, false, Some(&converted_images))?;
                let mut ctx = TeraContext::new();
                ctx.insert("config", &config);
                ctx.insert("current_page", &page);
                ctx.insert("content", &page.content_html);
                ctx.insert("root_path", &root_path_for_url(&page.url));
                ctx.insert("current_url", &page.url);

                let render_out = tera.render("page.html", &ctx)?;
                let page_path = output_dir.join(&page.url);
                if let Some(parent) = page_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(page_path, render_out)?;
            }
            "link" => {}
            _ => {}
        }
    }

    // Generate blog pages from content/blog/
    let blog_posts = scan_blog_posts(content_dir, Some(&converted_images))?;
    if !blog_posts.is_empty() {
        println!(
            "{} {} {}",
            "Found".cyan(),
            blog_posts.len().to_string().cyan(),
            "blog posts".cyan()
        );

        // Render individual blog post pages
        for post in &blog_posts {
            let mut ctx = TeraContext::new();
            ctx.insert("config", &config);
            ctx.insert("current_page", &post);
            ctx.insert("content", &post.content_html);
            ctx.insert("root_path", &root_path_for_url(&post.url));
            ctx.insert("current_url", &post.url);

            let render_out = tera.render("post.html", &ctx)?;
            let post_path = output_dir.join(&post.url);
            if let Some(parent) = post_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&post_path, render_out)?;
            println!(
                "  {} {}",
                "Generated:".green(),
                post_path.display().to_string().green()
            );
        }

        // Render blog listing page
        let mut ctx = TeraContext::new();
        ctx.insert("config", &config);
        ctx.insert("posts", &blog_posts);
        ctx.insert("root_path", ".");
        ctx.insert("current_url", "blog.html");

        let render_out = tera.render("blog.html", &ctx)?;
        let blog_path = output_dir.join("blog.html");
        fs::write(&blog_path, render_out)?;
        println!(
            "  {} {}",
            "Generated blog index:".green(),
            blog_path.display().to_string().green()
        );

        let rss_path = output_dir.join("rss.xml");
        fs::write(&rss_path, generate_rss(&config, &blog_posts))?;
        println!(
            "  {} {}",
            "Generated RSS feed:".green(),
            rss_path.display().to_string().green()
        );

        // Generate search index JSON for full-text search
        if !blog_posts.is_empty() {
            #[derive(Serialize)]
            struct SearchDocument {
                title: Option<String>,
                date: Option<String>,
                url: String,
                content: String,
            }

            let mut search_docs = Vec::new();
            for post in &blog_posts {
                let plain_content = strip_html_tags(&post.content_html);
                search_docs.push(SearchDocument {
                    title: post.meta.title.clone(),
                    date: post.meta.date.clone(),
                    url: post.url.clone(),
                    content: plain_content,
                });
            }

            let search_json = serde_json::to_string_pretty(&search_docs)?;
            let search_index_path = output_dir.join("search_index.json");
            fs::write(&search_index_path, search_json)?;
            println!(
                "  {} {}",
                "Generated search index:".green(),
                search_index_path.display().to_string().green()
            );
        }
    }

    println!("{}", "Build success!".green().bold());
    Ok(())
}

/// Very simple HTML tag stripper for search indexing
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => result.push(c),
            _ => {}
        }
    }
    // Collapse multiple whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn validate_output_dir(output_dir: &Path, content_dir: &Path, theme_dir: &Path) -> Result<()> {
    if output_dir.as_os_str().is_empty() {
        bail!("output_dir cannot be empty");
    }

    // Reject obvious dangerous values before canonicalization
    let output_str = output_dir.to_str().unwrap_or("");
    if output_str == "." || output_str == ".." || output_str.starts_with("../") {
        bail!(
            "Refusing to use '{}' as output_dir (would delete project root)",
            output_dir.display()
        );
    }

    let cwd = std::env::current_dir().context("Failed to resolve current directory")?;
    let cwd_abs = absolutize_for_guard(&cwd, &cwd);
    let output_abs = absolutize_for_guard(&cwd, output_dir);
    let protected = [
        cwd_abs.clone(),
        absolutize_for_guard(&cwd, Path::new(".")),
        absolutize_for_guard(&cwd, content_dir),
        absolutize_for_guard(&cwd, theme_dir),
    ];

    if protected.iter().any(|path| path == &output_abs) {
        bail!(
            "Refusing to use protected directory '{}' as output_dir",
            output_dir.display()
        );
    }

    if !output_abs.starts_with(&cwd_abs) {
        bail!(
            "Refusing to write output_dir '{}' outside the project directory",
            output_dir.display()
        );
    }

    Ok(())
}

fn root_path_for_url(url: &str) -> String {
    let depth = Path::new(url)
        .parent()
        .map(|parent| parent.components().count())
        .unwrap_or(0);

    if depth == 0 {
        ".".to_string()
    } else {
        std::iter::repeat_n("..", depth)
            .collect::<Vec<_>>()
            .join("/")
    }
}

fn absolutize_for_guard(cwd: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    let resolved = joined.canonicalize().unwrap_or(joined);
    let mut normalized = PathBuf::new();
    for component in resolved.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
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
