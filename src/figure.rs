//! Compiling one lini block into the figure that replaces it.

use std::path::Path;

use lini::{Diagnostic, Level, Options};

/// Compile one block's source into its `<div class="lini-figure">`.
///
/// A block that fails to compile becomes a visible error box and the message
/// goes to stderr — the build still finishes, so one bad diagram never costs
/// you the rest of the book.
pub fn render(source: &str, chapter: &str, first_line: usize, base_dir: Option<&Path>) -> String {
    // Lini counts lines from the top of what it is handed, so pad the source
    // with the markdown standing above it. Diagnostics then read
    // `chapter.md:LINE:COL` against the real file.
    let padded = "\n".repeat(first_line - 1) + source;
    let options = Options { base_dir: base_dir.map(Path::to_path_buf), ..Options::default() };

    // Validation first: an unknown property or a malformed value costs us the
    // figure, exactly as it would on the command line.
    if let Some(fatal) = report(lini::lint_str(&padded).unwrap_or_default(), &padded, chapter) {
        return error_box(&fatal);
    }
    match lini::compile_str_checked(&padded, &options) {
        // Routing relaxations are warnings — the figure still draws.
        Ok((svg, routing)) => {
            report(routing, &padded, chapter);
            wrap(&svg)
        }
        Err(e) => {
            let text = e.display_with_source(&padded, chapter).to_string();
            eprintln!("mdbook-lini: {text}");
            error_box(&text)
        }
    }
}

/// Print every diagnostic; return the first error-level one, which is fatal.
fn report(diags: Vec<Diagnostic>, source: &str, chapter: &str) -> Option<String> {
    let mut fatal = None;
    for d in diags {
        let text = d.display_with_source(source, chapter).to_string();
        eprintln!("mdbook-lini: {text}");
        if d.level == Level::Error && fatal.is_none() {
            fatal = Some(text);
        }
    }
    fatal
}

/// Wrap a compiled SVG in the figure div the stylesheet targets.
///
/// The wrapper carries `--lini-w`, the diagram's natural pixel width — the one
/// hook the stylesheet needs to floor how far a wide figure may scale down
/// before the wrapper scrolls instead.
fn wrap(svg: &str) -> String {
    match natural_width(svg) {
        Some(w) => format!("<div class=\"lini-figure\" style=\"--lini-w: {w}px\">{svg}</div>"),
        None => format!("<div class=\"lini-figure\">{svg}</div>"),
    }
}

/// The `width` lini bakes onto the root `<svg>` tag, in pixels.
fn natural_width(svg: &str) -> Option<&str> {
    let tag = &svg[..svg.find('>')?];
    let (_, after) = tag.split_once(" width=\"")?;
    after.split_once('"').map(|(width, _)| width)
}

fn error_box(message: &str) -> String {
    format!("<pre class=\"lini-error\">{}</pre>", escape(message))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_a_diagram_with_its_natural_width() {
        let html = render("a -> b", "demo.md", 1, None);
        assert!(html.starts_with("<div class=\"lini-figure\" style=\"--lini-w: "), "{html}");
        assert!(html.contains("<svg") && html.ends_with("</div>"));
    }

    #[test]
    fn a_broken_block_becomes_an_error_box() {
        let html = render("|box", "demo.md", 1, None);
        assert!(html.starts_with("<pre class=\"lini-error\">"), "{html}");
    }

    #[test]
    fn an_error_points_at_the_chapter_line_not_the_block_line() {
        let html = render("a -> b\n|box", "demo.md", 41, None);
        assert!(html.contains("demo.md:42:"), "{html}");
    }

    #[test]
    fn reads_the_width_off_the_root_tag() {
        assert_eq!(natural_width(r#"<svg viewBox="0 0 8 4" width="8" height="4">"#), Some("8"));
        assert_eq!(natural_width("<svg>"), None);
    }
}
