# mdBear

A fast static site generator inspired by Bear Blog, written in Rust.

## Features

- Generate static sites from Markdown files
- Support for both pages and blog posts with date-based ordering
- Customizable themes using Tera templating engine
- Simple initialization command to get started quickly

## Installation

```bash
cargo install mdbear
```

## Usage

```bash
# Initialize a new site
mdbear init [your-site]

# Build the site
mdbear build

# Serve locally with auto-reload (default: port 3000)
mdbear serve

# Sync theme files from defaults
mdbear sync
```

## License

[MIT](./LICENSE)
