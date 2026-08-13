//! mdbook preprocessor that renders ```` ```lini ```` fenced blocks to inline SVG.
//!
//! [Lini](https://lini.rs) is one engine for every figure family: flowcharts,
//! charts, sequences, mindmaps, trees, schematics, and technical drawings all
//! come out of the same fence. Lini is a library here, not a subprocess —
//! installing this binary is the whole toolchain.
//!
//! Each block is replaced with a `<div class="lini-figure">` wrapping the SVG.
//! Colours stay live `var(--lini-*)` references, so a figure follows the book's
//! light/dark toggle through CSS alone — see `mdbook-lini.css`, which rides
//! along in a `<style>` block rather than asking the book to link it.

mod css;
mod fence;
mod figure;

use std::error::Error;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

fn main() -> Result<(), Box<dyn Error>> {
    // mdbook probes support with `<preprocessor> supports <renderer>`.
    let argv: Vec<String> = std::env::args().collect();
    if argv.get(1).map(String::as_str) == Some("supports") {
        let html = argv.get(2).map(String::as_str) == Some("html");
        std::process::exit(if html { 0 } else { 1 });
    }

    // The preprocessor input is the JSON pair [context, book].
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;
    let mut payload: Vec<Value> = serde_json::from_str(&raw)?;
    let mut book = payload.pop().ok_or("preprocessor input missing book")?;
    let context = payload.pop().ok_or("preprocessor input missing context")?;

    let src_root = src_root(&context);
    let bundled_css = bundled_css(&context);
    if let Some(sections) = sections_mut(&mut book) {
        render(sections, &src_root, bundled_css);
    }

    let mut out = BufWriter::new(std::io::stdout().lock());
    serde_json::to_writer(&mut out, &book)?;
    out.flush()?;
    Ok(())
}

/// The book's chapter list — `sections` in current mdbook, `items` in older.
fn sections_mut(book: &mut Value) -> Option<&mut Vec<Value>> {
    let obj = book.as_object_mut()?;
    let key = if obj.contains_key("sections") { "sections" } else { "items" };
    obj.get_mut(key)?.as_array_mut()
}

/// The book's source directory, which a chapter's `source_path` hangs off.
fn src_root(context: &Value) -> PathBuf {
    let root = context["root"].as_str().unwrap_or(".");
    let src = context["config"]["book"]["src"].as_str().unwrap_or("src");
    Path::new(root).join(src)
}

/// Whether to ship our own stylesheet with the figures — `bundled-css` in
/// `[preprocessor.lini]`, on unless the book turns it off. This governs the
/// figure wrapper and the theme binding only; the styling *inside* each SVG is
/// Lini's, and travels with it either way.
fn bundled_css(context: &Value) -> bool {
    context["config"]["preprocessor"]["lini"]["bundled-css"].as_bool().unwrap_or(true)
}

/// Recursively rewrite every chapter's lini blocks.
fn render(items: &mut [Value], src_root: &Path, bundled_css: bool) {
    for item in items {
        let Some(chapter) = item.get_mut("Chapter").and_then(Value::as_object_mut) else {
            continue;
        };
        let source_path =
            chapter.get("source_path").and_then(Value::as_str).unwrap_or("<book>").to_owned();
        // A local `|image| src:` resolves against the chapter's own directory.
        let base_dir = src_root.join(&source_path).parent().map(Path::to_path_buf);

        if let Some(content) = chapter.get("content").and_then(Value::as_str) {
            let mut figures = 0;
            let mut rendered = fence::rewrite(content, |source, line| {
                figures += 1;
                figure::render(source, &source_path, line, base_dir.as_deref())
            });
            // Only a chapter that drew something needs the stylesheet.
            if bundled_css && figures > 0 {
                rendered.insert_str(0, css::style_tag());
            }
            chapter.insert("content".into(), Value::String(rendered));
        }
        if let Some(sub) = chapter.get_mut("sub_items").and_then(Value::as_array_mut) {
            render(sub, src_root, bundled_css);
        }
    }
}
