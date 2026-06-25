use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn vibe_fixture_builds_editorial_site() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("test").join("vibe-demo");
    let workdir = temp_project_dir();

    copy_dir_all(&fixture, &workdir);

    let output = Command::new(env!("CARGO_BIN_EXE_mdbear"))
        .arg("build")
        .current_dir(&workdir)
        .output()
        .expect("failed to run mdbear build");

    assert!(
        output.status.success(),
        "build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let index = read_output(&workdir, "index.html");
    let blog = read_output(&workdir, "blog.html");
    let about = read_output(&workdir, "about.html");
    let post = read_output(&workdir, "blog/tufted-demo.html");
    let rss = read_output(&workdir, "rss.xml");
    let css = read_output(&workdir, "style.css");

    assert_contains(&index, "site-shell");
    assert_contains(&index, "site-rail");
    assert_contains(&index, "page-margin");
    assert_contains(&index, "toc-card");
    assert_contains(&index, "sidenote");
    assert_contains(&index, "fa-solid fa-house");
    assert_contains(&index, "fa-solid fa-circle-half-stroke");
    assert_contains(&index, "nav-theme-toggle");
    assert_contains(&index, "immersive-light");
    assert_contains(&index, "pointermove");
    assert_contains(&index, "application/rss+xml");
    assert_contains(&index, "./rss.xml");

    assert_contains(&blog, "blog-index");
    assert_contains(&blog, "Three-Column Layout Test");

    assert_contains(
        &about,
        "<aside class=\"sidenote\"><span class=\"sidenote-num\">",
    );
    assert!(
        workdir
            .join("mdbear")
            .join("assets")
            .join("images")
            .join("magicsquash.webp")
            .exists()
    );

    assert_contains(&post, "<details class=\"toc-card immersive-light\">");
    assert_contains(&post, "toc-level-2");
    assert_contains(&post, "#design-goals");
    assert_contains(&post, "typst-block");
    assert_contains(&post, "Hello Typst");
    assert_contains(&post, "fa-solid fa-pen-nib");
    assert_contains(&post, "../rss.xml");
    assert_contains(
        &post,
        "sidenote-marker",
    );

    assert_contains(&rss, "<rss version=\"2.0\"");
    assert_contains(&rss, "<title>Vibe Fixture</title>");
    assert_contains(&rss, "<link>https://example.com</link>");
    assert_contains(&rss, "<item>");
    assert_contains(&rss, "<title>Three-Column Layout Test</title>");
    assert_contains(
        &rss,
        "<link>https://example.com/blog/tufted-demo.html</link>",
    );
    assert_contains(&rss, "<pubDate>Wed, 24 Jun 2026 00:00:00 +0000</pubDate>");

    assert_contains(
        &css,
        "grid-template-columns: minmax(7rem, 0.45fr) minmax(0, 42rem) minmax(14rem, 0.9fr)",
    );
    assert_contains(&css, ".immersive-light");
    assert_contains(&css, "--glow-x: 50%");
    assert_contains(
        &css,
        "backdrop-filter: blur(29px) saturate(1.42) contrast(1.06)",
    );
    assert_contains(&css, "width: 4.1rem");
    assert_contains(&css, "top: 50%");
    assert_contains(&css, "transform: translateY(-50%)");
    assert_contains(&css, "bottom: max(0.75rem, env(safe-area-inset-bottom))");
    assert_contains(&css, "display: none");
    assert_contains(&css, ".sidenote");
    assert_contains(&css, ".sidenote img");
    assert_contains(&css, "@media (max-width: 1080px)");

    fs::remove_dir_all(&workdir).ok();
}

fn temp_project_dir() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis();
    std::env::temp_dir().join(format!(
        "mdbear-vibe-fixture-{}-{}",
        std::process::id(),
        millis
    ))
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("failed to create destination directory");
    for entry in fs::read_dir(src).expect("failed to read source directory") {
        let entry = entry.expect("failed to read directory entry");
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry
            .file_type()
            .expect("failed to read file type")
            .is_dir()
        {
            copy_dir_all(&src_path, &dst_path);
        } else {
            fs::copy(&src_path, &dst_path).unwrap_or_else(|_| {
                panic!(
                    "failed to copy {} to {}",
                    src_path.display(),
                    dst_path.display()
                )
            });
        }
    }
}

fn read_output(workdir: &Path, relative: &str) -> String {
    fs::read_to_string(workdir.join("mdbear").join(relative))
        .unwrap_or_else(|_| panic!("missing output file: {}", relative))
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "missing expected content: {}",
        needle
    );
}
