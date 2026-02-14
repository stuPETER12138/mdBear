# mdBear

A fast static site generator inspired by Bear Blog, written in Rust. Create beautiful, lightweight websites from Markdown files with customizable themes.

## Features

- Generate static sites from Markdown files with YAML frontmatter
- Support for both pages and blog posts with date-based ordering
- Customizable themes using Tera templating engine
- Built-in development server with live preview
- Asset copying for images, CSS, JavaScript, and other resources
- Simple initialization command to get started quickly

## Installation

Install from crates.io:
```bash
cargo install mdbear
```

Or build from source:
```bash
git clone https://github.com/stuPETER12138/mdBear
cd mdBear
cargo install --path .
```

## Usage

### Initialize a new site
```bash
mdbear init mysite
```

This creates a new directory with the default structure:
- `config.toml` - Site configuration
- `content/` - Markdown content files
- `theme/` - HTML templates

### Build the site
```bash
mdbear build [options]
```
Options:
- `-c, --config <CONFIG>` - Configuration file (default: "config.toml")

### Serve with live preview
```bash
mdbear serve [options]
```
Options:
- `-p, --port <PORT>` - Port to serve on (default: 3000)
- `-c, --config <CONFIG>` - Configuration file (default: "config.toml")

## Configuration

The `config.toml` file defines your site structure:

```toml
site_icon = ":-)"
site_name = "My Site"
author = "Your Name"
output_dir = "dist"

[[nav]]
name = "Home"
path = "index.md"
type = "page"

[[nav]]
name = "Blog"
path = "blog"
type = "blog"
```

Navigation types:
- `page` - Individual pages like "About", "Contact"
- `blog` - Collections of blog posts organized by date
- `link` - External links

## Content Format

Pages and blog posts use Markdown with YAML frontmatter:

```yaml
---
title: My Blog Post
date: 2023-01-01
---

# My Content

This is my blog post content in Markdown.
```

## Theme System

mdBear uses Tera templates for theming. The default theme includes:
- `page.html` - Template for individual pages
- `list.html` - Template for blog index pages

## License

MIT