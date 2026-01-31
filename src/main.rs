use anyhow::Result;
use clap::Parser;
use gray_matter::{Matter, ParsedEntity, engine::YAML};
use pulldown_cmark::{Parser as MdParser, html};
use serde::{Deserialize, Serialize};
use tera::{Context as TeraContext, Tera};

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Config {
    site_name: String,
    author: String,
    output_dir: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct PostMeta {
    title: String,
    date: String,
}

struct Post {
    meta: PostMeta,
    content_html: String,
    slug: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config_content = std::fs::read_to_string(&args.config)?;
    let config: Config = toml::from_str(&config_content)?;

    println!("Load config: {:?}", config);

    let tera = Tera::new("templates/**/*.html")?;
    println!(
        "Templates loaded: {:?}",
        tera.get_template_names().collect::<Vec<_>>()
    );

    let output_dir = std::path::Path::new(&config.output_dir);
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir)?;
    }
    std::fs::create_dir_all(output_dir)?;

    let content_dir = std::path::Path::new("content");
    let mut posts = Vec::new();

    for entry in content_dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            println!("Processing: {:?}", path);
            let post = process_post(&path)?;

            let mut context = TeraContext::new();
            context.insert("config", &config);
            context.insert("meta", &post.meta);
            context.insert("content", &post.content_html);

            let rendered = tera.render("post.html", &context)?;

            let output_path = output_dir.join(format!("{}.html", post.slug));
            std::fs::write(&output_path, rendered)?;

            posts.push(post);
        }
    }

    println!(
        "Build successfully! Generated {} pages in '{}'",
        posts.len(),
        config.output_dir
    );
    Ok(())
}

fn process_post(path: &std::path::Path) -> Result<Post> {
    let content = std::fs::read_to_string(path)?;
    let matter = Matter::<YAML>::new();
    let result: ParsedEntity = matter.parse(&content)?;
    let meta: PostMeta = result.data.unwrap().deserialize()?;
    let parser = MdParser::new(&result.content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    let slug = path.file_stem().unwrap().to_str().unwrap().to_string();
    Ok(Post {
        meta,
        content_html: html_output,
        slug,
    })
}
