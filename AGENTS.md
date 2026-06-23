# AGENTS.md

## Commands

- Build/check/lint/test from repo root: `cargo build`, `cargo check`, `cargo clippy`, `cargo test`.
- Run the CLI from source: `cargo run -- build`, `cargo run -- serve --port 3000`, `cargo run -- init my-site`, `cargo run -- sync`.
- Focused integration test: `cargo test --test vibe_fixture`.
- Manual fixture build: `cargo run -- build -c test/vibe-demo/config.toml` is not how this app resolves paths; instead run the binary from inside a site directory, e.g. `target/debug/mdbear.exe build` with cwd `test/vibe-demo`.

## Architecture notes

- Binary entrypoint is `src/main.rs`; Clap command definitions are in `src/cli.rs`.
- Subcommands live in `src/cmd/`: `build`, `serve`, `init`, `sync`.
- Build assumes site-relative directories from the current working directory: `config.toml`, `content/`, `theme/`, and writes `config.output_dir`.
- `src/cmd/build.rs` renders `.md` nav pages only; non-`.md` nav pages such as `blog.html` are skipped there and generated separately from `content/blog/`.
- Markdown parsing and site content transforms are in `src/utils.rs`: frontmatter, Markdown options, TOC collection, sidenotes, Typst fenced blocks, Font Awesome shorthands, image extension rewriting.
- Default scaffold/theme files are embedded from `defaults/` via `rust-embed`; update `defaults/theme/*` when changing generated site UI.

## Theme/content conventions

- Tera templates: `defaults/theme/base.html`, `page.html`, `post.html`, `blog.html`; CSS is `defaults/theme/style.css`.
- Right-side TOC uses `current_page.toc` generated from H2/H3 headings and is rendered as collapsed `<details>`.
- Sidenotes use Markdown text `[^side: note text]` and render as `.sidenote`.
- Typst support is source display for fenced ```typst blocks, rendered as `.typst-block`; it does not compile Typst to PDF/SVG.
- Font Awesome shorthand is `:fa-icon-name:` and renders as `fa-solid fa-icon-name`; the default theme loads Font Awesome from CDN.

## Tests and fixtures

- Integration fixture lives at `test/vibe-demo/`; its generated output `test/*/mdbear/` is ignored.
- `tests/vibe_fixture.rs` copies `test/vibe-demo` to a temp dir, runs `CARGO_BIN_EXE_mdbear build`, then asserts generated HTML/CSS details.
- When changing `defaults/theme/*`, sync the fixture theme too: `Copy-Item -Path "defaults\\theme\\*" -Destination "test\\vibe-demo\\theme" -Recurse -Force`.
- Before finishing code changes, run `cargo test`, `cargo check`, and `cargo clippy`.
