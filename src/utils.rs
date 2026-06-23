use anyhow::{Context, Result};
use colored::Colorize;
use gray_matter::{Matter, ParsedEntity, engine::YAML};
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::Lanczos3};
use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser as MdParser, Tag, TagEnd, html};
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub site_icon: String,
    pub site_name: String,
    pub author: String,
    #[serde(default)]
    pub footer: Option<String>,
    pub output_dir: String,
    pub blog_url: Option<String>,
    #[serde(default)]
    pub site_description: Option<String>,
    #[serde(default)]
    pub social: SocialLinks,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub blog: BlogConfig,
    pub nav: Vec<NavItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct SocialLinks {
    pub github: Option<String>,
    pub twitter: Option<String>,
    pub linkedin: Option<String>,
    pub email: Option<String>,
    pub scholar: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ThemeConfig {
    pub mode: Option<String>, // "light", "dark", or "auto"
    pub color_scheme: Option<String>, // "catppuccin-latte", "catppuccin-frappe", etc.
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct BlogConfig {
    pub posts_per_page: Option<usize>,
    pub sort_by: Option<String>, // "date", "title"
    pub sort_order: Option<String>, // "asc", "desc"
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NavItem {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    // item type: "page" (internal page), "link" (external website), "file" (internal file link)
    pub item_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct PostMeta {
    pub title: Option<String>,
    pub date: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TocItem {
    pub level: u8,
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Page {
    pub meta: PostMeta,
    pub content_html: String,
    pub toc: Vec<TocItem>,
    pub slug: String,
    pub url: String,
}

#[derive(RustEmbed)]
#[folder = "defaults/"]
pub struct DefaultAssets;

// general functions

pub fn load_page(base_content_dir: &Path, file_path: &str, strict_mode: bool) -> Result<Page> {
    let full_path = base_content_dir.join(file_path);
    let content = fs::read_to_string(&full_path)
        .with_context(|| format!("Cannot read file: {:?}", full_path))?;

    let matter = Matter::<YAML>::new();
    let result: ParsedEntity = matter.parse(&content)?;

    let mut meta: PostMeta = if let Some(data) = result.data {
        data.deserialize().unwrap_or_default()
    } else {
        if strict_mode {
            return Err(anyhow::anyhow!(
                "{:?} is missing Frontmatter (title, date)",
                full_path
            ));
        }
        PostMeta::default()
    };

    let stem = Path::new(file_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    if strict_mode && meta.title.is_none() {
        meta.title = Some(stem.clone());
    }

    if strict_mode && meta.date.is_none() {
        return Err(anyhow::anyhow!(
            "Blog {:?} is missing 'date' field",
            full_path
        ));
    }

    let (processed_content, math_blocks) = protect_math(&result.content);
    let (processed_content, typst_blocks) = protect_typst(&processed_content);
    let parser = MdParser::new_ext(&processed_content, markdown_options());
    let (events, toc) = collect_toc(parser);
    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());
    let html_output = restore_typst(&html_output, &typst_blocks);
    let html_output = restore_math(&html_output, &math_blocks);
    let html_output = render_sidenotes(&html_output);
    let html_output = render_fontawesome(&html_output);

    let re = Regex::new(r#"(src=["'][^"']*)\.(png|jpg|jpeg|gif)(["'])"#).unwrap();
    let html_output = re.replace_all(&html_output, "${1}.webp${3}").to_string();

    let url = if stem == "index" && !file_path.contains('/') {
        "index.html".to_string()
    } else {
        format!("{}.html", stem)
    };

    Ok(Page {
        meta,
        content_html: html_output,
        toc,
        slug: stem,
        url,
    })
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

fn collect_toc<'a>(parser: MdParser<'a>) -> (Vec<Event<'a>>, Vec<TocItem>) {
    let mut events = Vec::new();
    let mut toc = Vec::new();
    let mut heading: Option<(HeadingLevel, String, String)> = None;
    let mut slugs: HashMap<String, usize> = HashMap::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, id, classes, attrs }) => {
                heading = Some((level, id.as_ref().map(|s| s.to_string()).unwrap_or_default(), String::new()));
                events.push(Event::Start(Tag::Heading { level, id, classes, attrs }));
            }
            Event::Text(text) => {
                if let Some((_, _, title)) = &mut heading {
                    title.push_str(&text);
                }
                events.push(Event::Text(text));
            }
            Event::Code(text) => {
                if let Some((_, _, title)) = &mut heading {
                    title.push_str(&text);
                }
                events.push(Event::Code(text));
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some((heading_level, id, title)) = heading.take()
                    && matches!(heading_level, HeadingLevel::H2 | HeadingLevel::H3)
                {
                    let final_id = if id.is_empty() {
                        unique_slug(&title, &mut slugs)
                    } else {
                        id
                    };
                    toc.push(TocItem {
                        level: heading_level_number(heading_level),
                        id: final_id.clone(),
                        title,
                    });
                    events.push(Event::Html(CowStr::from(format!(
                        "<a class=\"heading-anchor\" id=\"{}\"></a>",
                        final_id
                    ))));
                }
                events.push(Event::End(TagEnd::Heading(level)));
            }
            _ => events.push(event),
        }
    }

    (events, toc)
}

fn heading_level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn unique_slug(title: &str, slugs: &mut HashMap<String, usize>) -> String {
    let mut slug = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if slug.is_empty() {
        slug = "section".to_string();
    }

    let count = slugs.entry(slug.clone()).or_insert(0);
    let result = if *count == 0 {
        slug.clone()
    } else {
        format!("{}-{}", slug, count)
    };
    *count += 1;
    result
}

fn protect_math(content: &str) -> (String, Vec<String>) {
    protect_blocks(content, "$$", "MDBEAR_MATH")
}

fn restore_math(content: &str, blocks: &[String]) -> String {
    restore_blocks(content, blocks, "MDBEAR_MATH", |block| {
        format!("<div class=\"math-block\">{}</div>", block)
    })
}

fn protect_typst(content: &str) -> (String, Vec<String>) {
    let mut output = String::new();
    let mut blocks = Vec::new();
    let mut rest = content;

    while let Some(start) = rest.find("```typst") {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + "```typst".len()..];
        if let Some(end) = after_start.find("```") {
            let index = blocks.len();
            blocks.push(after_start[..end].trim().to_string());
            output.push_str(&format!("\nMDBEAR_TYPST{}\n", index));
            rest = &after_start[end + "```".len()..];
        } else {
            output.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }

    output.push_str(rest);
    (output, blocks)
}

fn restore_typst(content: &str, blocks: &[String]) -> String {
    restore_blocks(content, blocks, "MDBEAR_TYPST", |block| {
        format!("<pre class=\"typst-block\"><code>{}</code></pre>", escape_html(block))
    })
}

fn protect_blocks(content: &str, delimiter: &str, prefix: &str) -> (String, Vec<String>) {
    let mut output = String::new();
    let mut blocks = Vec::new();
    let mut rest = content;

    while let Some(start) = rest.find(delimiter) {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + delimiter.len()..];
        if let Some(end) = after_start.find(delimiter) {
            let block = format!("{}{}{}", delimiter, &after_start[..end], delimiter);
            let index = blocks.len();
            blocks.push(block);
            output.push_str(&format!("\n{}{}\n", prefix, index));
            rest = &after_start[end + delimiter.len()..];
        } else {
            output.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }

    output.push_str(rest);
    (output, blocks)
}

fn restore_blocks<F>(content: &str, blocks: &[String], prefix: &str, render: F) -> String
where
    F: Fn(&str) -> String,
{
    let mut output = content.to_string();
    for (index, block) in blocks.iter().enumerate() {
        output = output.replace(&format!("<p>{}{}</p>", prefix, index), &render(block));
        output = output.replace(&format!("{}{}", prefix, index), &render(block));
    }
    output
}

fn render_sidenotes(content: &str) -> String {
    let re = Regex::new(r#"\[\^side:\s*([^\]]+)\]"#).unwrap();
    re.replace_all(content, r#"<span class="sidenote">$1</span>"#).to_string()
}

fn render_fontawesome(content: &str) -> String {
    let re = Regex::new(r#":fa-([a-z0-9-]+):"#).unwrap();
    re.replace_all(content, r#"<i class="fa-solid fa-$1" aria-hidden="true"></i>"#).to_string()
}

fn escape_html(content: &str) -> String {
    content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn scan_blog_posts(content_dir: &Path) -> Result<Vec<Page>> {
    let blog_dir = content_dir.join("blog");
    if !blog_dir.exists() {
        return Ok(Vec::new());
    }

    let mut posts = Vec::new();
    let entries = std::fs::read_dir(&blog_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let relative = path
                .strip_prefix(content_dir)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            match load_page(content_dir, &relative, true) {
                Ok(mut page) => {
                    page.url = format!("blog/{}", page.url);
                    posts.push(page);
                }
                Err(e) => {
                    eprintln!(
                        "  {} {}: {}",
                        "跳过".yellow(),
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    // Sort by date descending (newest first)
    posts.sort_by(|a, b| {
        let a_date = a.meta.date.as_deref().unwrap_or("0");
        let b_date = b.meta.date.as_deref().unwrap_or("0");
        b_date.cmp(a_date)
    });

    Ok(posts)
}

pub fn images2webp(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    if !src_dir.exists() {
        return Ok(());
    }

    fs::create_dir_all(dst_dir)?;

    const MAX_WIDTH: u32 = 1201;

    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let sub_dst = dst_dir.join(entry.file_name());
            images2webp(&path, &sub_dst)?;
            continue;
        }

        if let Ok(img) = image::open(&path) {
            let (width, height) = img.dimensions();

            let resized: DynamicImage = if width > MAX_WIDTH {
                let new_height = (height as u64 * MAX_WIDTH as u64 / width as u64).max(1) as u32;
                img.resize(MAX_WIDTH, new_height, Lanczos3)
            } else {
                img
            };

            let new_filename = path.file_stem().unwrap().to_str().unwrap().to_string() + ".webp";
            let dst_path = dst_dir.join(&new_filename);

            resized.write_to(
                &mut std::io::BufWriter::new(fs::File::create(&dst_path)?),
                ImageFormat::WebP,
            )?;

            fs::remove_file(&path)?;

            println!(
                "{} {} -> {} ({}x{} -> {}x{})",
                "Converted:".cyan(),
                path.display(),
                dst_path.display(),
                width,
                height,
                resized.width(),
                resized.height()
            );
        }
    }

    Ok(())
}
