use anyhow::{Context, Result};
use gray_matter::{Matter, ParsedEntity, engine::YAML};
use pulldown_cmark::{Parser as MdParser, html};
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
