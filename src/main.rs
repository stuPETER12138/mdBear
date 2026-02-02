use anyhow::{Context, Result};
use clap::Parser;
use gray_matter::{Matter, ParsedEntity, engine::YAML};
use pulldown_cmark::{Parser as MdParser, html};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

// --- 1. 数据结构定义 ---

// 导航项配置
#[derive(Deserialize, Serialize, Debug, Clone)]
struct NavItem {
    name: String,
    path: String,
    #[serde(rename = "type")]
    item_type: String, // "page", "blog", "link"
}

// 全局配置
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Config {
    site_icon: String,
    site_name: String,
    author: String,
    output_dir: String,
    nav: Vec<NavItem>,
}

// 命令行参数
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

// 文章元数据 (Frontmatter)
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
struct PostMeta {
    title: Option<String>,
    date: Option<String>, // 对于 Blog 是必须的，对于 Page 是可选的
}

// 内存中的页面对象
#[derive(Debug, Serialize, Clone)]
struct Page {
    meta: PostMeta,
    content_html: String, // 转换后的 HTML 内容
    slug: String,         // 文件名 (不含后缀)
    url: String,          // 生成的 HTML 文件名 (例如 experience.html)
}

// --- 2. 核心处理逻辑 ---

/// 加载并解析 Markdown 文件
/// strict_mode: 如果为 true (Blog模式)，则必须包含 date，否则报错
fn load_page(base_content_dir: &Path, file_path: &str, strict_mode: bool) -> Result<Page> {
    let full_path = base_content_dir.join(file_path);
    let content =
        fs::read_to_string(&full_path).with_context(|| format!("无法读取文件: {:?}", full_path))?;

    // 1. 解析 YAML Frontmatter
    let matter = Matter::<YAML>::new();
    let result: ParsedEntity = matter.parse(&content)?;

    // 2. 处理元数据
    let mut meta: PostMeta = if let Some(data) = result.data {
        data.deserialize().unwrap_or_default()
    } else {
        if strict_mode {
            return Err(anyhow::anyhow!(
                "文件 {:?} 缺少必需的 Frontmatter (title, date)",
                full_path
            ));
        }
        PostMeta::default()
    };

    // 3. 智能填充标题：如果 YAML 里没写 title，就用文件名
    let stem = Path::new(file_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 4. Blog 模式下的严格检查
    if strict_mode && meta.date.is_none() {
        return Err(anyhow::anyhow!(
            "Blog 文章 {:?} 必须包含 'date' 字段",
            full_path
        ));
    }

    // 5. Markdown 转 HTML
    let parser = MdParser::new(&result.content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // 6. 计算 URL
    // index.md -> index.html
    // other.md -> other.html
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

// 递归复制文件夹 (用于 assets)
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
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

fn main() -> Result<()> {
    // A. 初始化
    let args = Args::parse();
    println!("加载配置: {}", args.config);

    let config_str = fs::read_to_string(&args.config)?;
    let config: Config = toml::from_str(&config_str)?;

    let output_dir = Path::new(&config.output_dir);
    let content_dir = Path::new("content");

    // 初始化 Tera 模板
    let tera = Tera::new("theme/**/*.html")?;

    // 清理并重建输出目录
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    // B. 资源处理 (Assets & Static)

    // 1. 复制 static (CSS 等)
    let static_src = Path::new("static");
    if static_src.exists() {
        println!("复制 Static 目录...");
        copy_dir_all(static_src, output_dir)?;
    }

    // 2. 复制 content/assets (图片, PDF) -> public/assets
    let assets_src = content_dir.join("assets");
    let assets_dst = output_dir.join("assets");
    if assets_src.exists() {
        println!("复制 Assets 目录...");
        // 确保 assets 目录结构被保留
        copy_dir_all(&assets_src, &assets_dst)?;
    }

    // C. 核心循环：根据 Config 生成页面
    for item in &config.nav {
        println!("正在构建: [{}] ({})", item.name, item.item_type);

        match item.item_type.as_str() {
            // --- 类型 1: 单页 (Page) ---
            // 适用于 Home(index.md), Experience, Publications
            "page" => {
                // strict_mode = false: 允许没有日期
                let page = load_page(content_dir, &item.path, false)?;

                let mut ctx = TeraContext::new();
                ctx.insert("config", &config);
                ctx.insert("current_page", &page);
                ctx.insert("content", &page.content_html);
                ctx.insert("root_path", "."); // 根目录下

                // 渲染并写入
                let render_out = tera.render("page.html", &ctx)?;
                fs::write(output_dir.join(&page.url), render_out)?;
            }

            // --- 类型 2: 博客 (Blog) ---
            // 适用于 blog/ 文件夹，自动生成列表
            "blog" => {
                let section_path = content_dir.join(&item.path);
                let section_out_dir = output_dir.join(&item.path);
                fs::create_dir_all(&section_out_dir)?;

                let mut posts = Vec::new();

                // 遍历 blog 文件夹
                for entry in fs::read_dir(&section_path)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.extension().map_or(false, |e| e == "md") {
                        // 构造相对路径: blog/xxx.md
                        let rel_path = format!(
                            "{}/{}",
                            item.path,
                            path.file_name().unwrap().to_str().unwrap()
                        );

                        // strict_mode = true: 必须有日期
                        match load_page(content_dir, &rel_path, true) {
                            Ok(page) => {
                                // 渲染单个博客文章
                                let mut ctx = TeraContext::new();
                                ctx.insert("config", &config);
                                ctx.insert("current_page", &page);
                                ctx.insert("content", &page.content_html);
                                ctx.insert("root_path", ".."); // 在 blog/ 子目录下，需要回退一级

                                let out_path = section_out_dir.join(format!("{}.html", page.slug));
                                let render_out = tera.render("page.html", &ctx)?;
                                fs::write(out_path, render_out)?;

                                posts.push(page);
                            }
                            Err(e) => {
                                eprintln!("  [警告] 跳过文件 {:?}: {}", path, e);
                            }
                        }
                    }
                }

                // 按日期降序排序 (最新的在前)
                posts.sort_by(|a, b| b.meta.date.cmp(&a.meta.date));

                // 渲染博客索引页 (blog/index.html)
                let mut ctx = TeraContext::new();
                ctx.insert("config", &config);
                ctx.insert("section_title", &item.name);
                ctx.insert("posts", &posts);
                ctx.insert("root_path", ".."); // 列表页也在 blog/ 下

                let render_out = tera.render("list.html", &ctx)?;
                fs::write(section_out_dir.join("index.html"), render_out)?;
            }

            // --- 类型 3: 链接 (Link) ---
            // 纯静态链接，无需生成 HTML，直接在模板导航栏中使用
            "link" => {
                println!("  -> 静态链接，跳过生成");
            }

            _ => println!("  -> 未知类型，忽略"),
        }
    }

    println!("构建完成！输出目录: {:?}", output_dir);
    Ok(())
}
