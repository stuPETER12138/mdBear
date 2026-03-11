use anyhow::{Context, Result};
use colored::Colorize;
use gray_matter::{Matter, ParsedEntity, engine::YAML};
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::Lanczos3};
use pulldown_cmark::{Parser as MdParser, html};
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub site_icon: String,
    pub site_name: String,
    pub author: String,
    pub output_dir: String,
    pub blog_url: Option<String>,
    pub nav: Vec<NavItem>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NavItem {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    // item type: "page", "blog", "link"
    pub item_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct PostMeta {
    pub title: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Page {
    pub meta: PostMeta,
    pub content_html: String,
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

    let parser = MdParser::new(&result.content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

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
