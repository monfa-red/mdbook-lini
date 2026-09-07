//! mdbook accepted the HTML the preprocessor emitted.
//!
//! The unit tests in `src/` check the markup we produce. This checks the thing
//! they cannot see: whether mdbook's Markdown parser kept it in one piece.
//!
//! The failure it guards is silent and total. A blank line ends an HTML block
//! in CommonMark, so a listing carrying one would spill the rest of the chapter
//! out of the figure and strand the closing tags at the foot of the page — with
//! no error from anything, and a book that still builds.
//!
//! Running it needs `mdbook` on PATH. Absent it, the test **skips with a note**
//! — except under `MDBOOK_REQUIRED=1`, where it fails instead. CI sets that, so
//! the skip can never quietly become the permanent state.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Skip unless CI demanded the run, in which case fail with the same reason.
fn unavailable(reason: &str) {
    if std::env::var("MDBOOK_REQUIRED").is_ok_and(|v| v == "1") {
        panic!("MDBOOK_REQUIRED=1 but {reason}");
    }
    eprintln!("skipping the book build: {reason}");
    eprintln!("  install it with `cargo install mdbook`");
}

fn have_mdbook() -> bool {
    Command::new("mdbook").arg("--version").output().is_ok_and(|o| o.status.success())
}

/// The text between each `open` and the `close` that follows it.
fn between_all<'a>(text: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    text.match_indices(open)
        .filter_map(|(at, _)| {
            let rest = &text[at + open.len()..];
            rest.find(close).map(|end| &rest[..end])
        })
        .collect()
}

/// Everything outside those runs, the delimiters included.
fn strip_between(text: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(open) {
        out.push_str(&rest[..at]);
        let after = &rest[at + open.len()..];
        match after.find(close) {
            Some(end) => rest = &after[end + close.len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// The first 200 characters, for a failure that has to quote the page.
fn head(text: &str) -> &str {
    text.char_indices().nth(200).map_or(text, |(at, _)| &text[..at])
}

/// PATH with the preprocessor's own directory in front: mdbook resolves
/// `[preprocessor.lini]` by looking up `mdbook-lini` by name.
fn path_with(dir: &Path) -> OsString {
    let Some(existing) = std::env::var_os("PATH") else {
        return dir.into();
    };
    let dirs = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&existing));
    std::env::join_paths(dirs).expect("rebuild PATH")
}

#[test]
fn mdbook_keeps_the_chapter_in_one_piece() {
    if !have_mdbook() {
        return unavailable("mdbook is not installed");
    }

    let root = repo_root();
    let bin = Path::new(env!("CARGO_BIN_EXE_mdbook-lini"));
    // Out of the fixture's own `build-dir`, so `cargo test` leaves the working
    // tree as it found it.
    let out = root.join("target/book");
    let build = Command::new("mdbook")
        .arg("build")
        .arg(root.join("tests/book"))
        .arg("--dest-dir")
        .arg(&out)
        .env("PATH", path_with(bin.parent().expect("the binary has a directory")))
        .output()
        .expect("run mdbook build");
    assert!(
        build.status.success(),
        "mdbook build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let page = out.join("chapter_1.html");
    let html = std::fs::read_to_string(&page).expect("read the built chapter");
    let Some(&main) = between_all(&html, "<main>", "</main>").first() else {
        panic!("no <main> in {}", page.display());
    };
    // Our stylesheet rides along inside the chapter and names every class we
    // look for below, so it has to go before anything is counted.
    let body = strip_between(main, "<style>", "</style>");

    let mut failures: Vec<String> = Vec::new();
    let mut want = |condition: bool, what: &str| {
        if !condition {
            failures.push(what.to_string());
        }
    };

    // Six lini blocks: two plain, one `figure-only`, one `code`, one
    // `code-only`, and one that does not compile.
    want(
        body.matches(r#"<div class="lini-figure-block">"#).count() == 3,
        "expected 3 figure blocks",
    );
    want(body.matches(r#"<div class="lini-source">"#).count() == 4, "expected 4 listings");
    want(body.matches(r#"class="lini-view-toggle""#).count() == 3, "expected 3 toggles");
    want(body.matches(r#"<div class="lini-figure""#).count() == 4, "expected 4 figures");

    // `code` puts the listing first and the figure behind the toggle. Found by
    // its label rather than by position, so reordering the fixture is harmless.
    let blocks = body.split(r#"<div class="lini-figure-block">"#).skip(1);
    let code_blocks: Vec<&str> = blocks.filter(|b| b.contains("Show figure")).collect();
    want(code_blocks.len() == 1, &format!("expected 1 `code` block, found {}", code_blocks.len()));
    for block in &code_blocks {
        want(
            block.find("lini-source") < block.find(r#"lini-figure""#),
            "the `code` block still leads with its figure",
        );
        want(
            block.find("lini-alt-view") < block.find(r#"lini-figure""#),
            "the figure is not the hidden half in `code` mode",
        );
    }

    // In `code` mode the control is inside mdbook's own button row, so mdbook
    // lays both out and the pointer never leaves the <pre> on its way to ours.
    // Placed on top instead, it drifts out of alignment and steals the hover
    // that keeps mdbook's copy button on screen.
    want(
        body.contains(r#"<pre><span class="buttons"><label class="lini-view-button""#),
        "the `code` control is not in mdbook's button row",
    );
    // The default mode has no <pre> on top to join, so it must not emit a row.
    want(body.matches(r#"class="buttons""#).count() == 1, "expected exactly 1 button row");

    // `code-only` is a listing and nothing else — no figure, no toggle, no
    // error box even though the fragment does not compile.
    want(
        body.matches("a shape, not a whole file").count() == 1,
        "the `code-only` fragment is missing",
    );

    // A <details> brings a disclosure marker a host stylesheet can re-assert
    // from outside our layer, and a second element for a theme to frame. The
    // caret and the box-in-a-box on lini.rs were both that element.
    want(!body.contains("<details"), "a <details> crept back in");
    want(!body.contains("<summary"), "a <summary> crept back in");

    // Every toggle id is unique and every label points at its own.
    let ids = between_all(&body, r#"class="lini-view-toggle" type="checkbox" id=""#, "\"");
    want(
        ids.iter().collect::<HashSet<_>>().len() == ids.len(),
        &format!("duplicate toggle ids: {ids:?}"),
    );
    for one in &ids {
        want(body.contains(&format!(r#"for="{one}""#)), &format!("no label points at {one}"));
    }
    want(body.contains(r#"class="nohighlight""#), "listing does not opt out of hljs");
    want(body.contains(r#"class="lini-tok-"#), "listing is not highlighted");

    // A block that failed to compile is on the page, and did not take the
    // page with it.
    want(body.matches(r#"<pre class="lini-error">"#).count() == 1, "expected 1 error box");

    // Every paragraph survived, in order.
    let prose =
        ["Prose after the chart, which must not be swallowed.", "One opted out with", "The end."];
    let mut at = 0;
    for text in prose {
        match body[at..].find(text) {
            Some(found) => at += found + text.len(),
            None => want(false, &format!("prose out of order or missing: {text:?}")),
        }
    }

    // The signature of a shattered HTML block: closing tags stranded after the
    // last prose, and paragraphs that were never meant to be paragraphs.
    let tail = &body[body.rfind("The end.").unwrap_or(0)..];
    want(
        !tail.contains("</pre>"),
        &format!("orphaned </pre> after the last prose: {:?}", head(tail)),
    );
    want(!tail.contains("</details>"), &format!("orphaned </details>: {:?}", head(tail)));

    // The source we emitted carries `&#10;` rather than newlines, so the block
    // scanner never sees a blank line. mdbook decodes those entities on the way
    // out — by then the block's extent is settled — so the reader gets the
    // author's own line breaks back. Both halves of that have to hold: the
    // listing shows the blank lines, and the chapter above stayed whole.
    let listings = between_all(&body, r#"<code class="nohighlight">"#, "</code>");
    want(listings.len() == 4, &format!("expected 4 listings, found {}", listings.len()));
    let spaced = listings.iter().filter(|text| strip_between(text, "<", ">").contains("\n\n"));
    want(spaced.count() == 1, "the blank-line source did not keep its blank lines");

    assert!(failures.is_empty(), "{}\n{}", page.display(), failures.join("\n"));
}
