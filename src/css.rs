//! The stylesheet the preprocessor ships with the figures it emits.
//!
//! A preprocessor cannot add a file to the book's output, but it can emit raw
//! HTML — so the styling rides along in a `<style>` block on each chapter that
//! actually has a figure. That is the whole setup: `[preprocessor.lini]` and
//! nothing else.
//!
//! It goes in `@layer mdbook-lini`, so any unlayered rule in the book's own CSS
//! wins without `!important`. Books that would rather own the styling outright
//! set `bundled-css = false` and link `mdbook-lini.css` themselves.

use std::sync::OnceLock;

/// The stylesheet, shared verbatim with the copy checked into the repo.
const SOURCE: &str = include_str!("../mdbook-lini.css");

/// The `<style>` block to prepend to a chapter, built once.
///
/// Two sheets ride along: ours, layered, and Lini's own token palette, taken
/// verbatim from [`lini::highlight_css`]. The palette is not copied here on
/// purpose — it is the stylesheet the highlighter's own markup is written
/// against, so a colour it adds or renames arrives with the compiler instead of
/// drifting until someone notices a listing has gone monochrome.
pub fn style_tag() -> &'static str {
    static TAG: OnceLock<String> = OnceLock::new();
    // `@layer a, b` fixes the order up front: Lini's own defaults sit below
    // ours, whatever order the SVGs below happen to declare them in. Without
    // this line every figure's `@layer lini.defaults` — declared later, and so
    // ranked higher — would outrank the `color-scheme` binding and strand the
    // book in light mode. Unlayered CSS still beats both, which is the point.
    //
    // The blank line keeps the chapter's own first line — a heading, usually —
    // out of the raw HTML block the style tag opens.
    TAG.get_or_init(|| {
        format!(
            "<style>@layer lini.defaults, mdbook-lini;\
             @layer mdbook-lini {{{}}}{}</style>\n\n",
            minify(SOURCE),
            // Lini's sheet declares its variables inside `@layer lini.defaults`
            // itself, so it goes outside our block — nesting it would rename
            // that layer and change what outranks what.
            minify(&lini::highlight_css())
        )
    })
}

/// Strip comments and collapse whitespace. The stylesheet is repeated once per
/// chapter, so it is worth the few hundred bytes.
fn minify(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return collapse(&out),
        }
    }
    out.push_str(rest);
    collapse(&out)
}

/// Squeeze every run of whitespace to one space, then drop the spaces that sit
/// beside punctuation and carry no meaning.
fn collapse(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    for part in css.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(part);
    }
    for sep in ['{', '}', ';', ':', ','] {
        for spaced in [format!(" {sep}"), format!("{sep} ")] {
            let bare = sep.to_string();
            while out.contains(&spaced) {
                out = out.replace(&spaced, &bare);
            }
        }
    }
    // The last declaration in a block does not need its separator.
    out.replace(";}", "}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tag_carries_the_stylesheet_in_its_own_layer() {
        let tag = style_tag();
        assert!(tag.starts_with("<style>@layer lini.defaults, mdbook-lini;"), "{tag}");
        assert!(tag.contains("@layer mdbook-lini {"), "{tag}");
        assert!(tag.ends_with("}</style>\n\n"));
        assert!(tag.contains(".lini-figure"));
        assert!(tag.contains("color-scheme"));

        // The ordering statement must precede the block it ranks, or it does
        // nothing at all.
        let order = tag.find("@layer lini.defaults,").unwrap();
        assert!(order < tag.find("@layer mdbook-lini {").unwrap());
    }

    /// The token palette is Lini's, not a copy of it here — this is what
    /// fails if someone re-adds one, or if the highlighter renames a class
    /// and the sheet stops matching its own markup.
    #[test]
    fn the_token_palette_comes_from_lini() {
        let tag = style_tag();
        assert!(tag.contains(".lini-tok-string"), "{tag}");
        assert!(tag.contains("--lini-tok-string"), "{tag}");
        assert!(!SOURCE.contains("tok-"), "the palette was copied back into our sheet");

        // A span the highlighter emits must be a selector the sheet paints.
        let html = lini::highlight_html("|box| \"hi\"");
        let class = html.split("<span class=\"").nth(1).expect("a span").split('"').next().unwrap();
        assert!(tag.contains(&format!(".{class}")), "no rule paints `{class}`: {tag}");
    }

    #[test]
    fn comments_and_slack_whitespace_are_dropped() {
        let out = minify("/* a note */\n.x {\n    color: red;\n}\n");
        assert_eq!(out, ".x{color:red}");
    }

    #[test]
    fn an_unterminated_comment_does_not_eat_what_came_before() {
        assert_eq!(minify(".x{color:red}\n/* oops"), ".x{color:red}");
    }

    /// A media query's condition needs the spaces the punctuation pass would
    /// otherwise be tempted to take.
    #[test]
    fn media_query_spacing_survives() {
        assert!(
            minify("@media only screen and (max-width: 768px) { .x { color: red } }")
                .contains("only screen and (max-width:768px)")
        );
    }

    #[test]
    fn the_shipped_stylesheet_minifies_to_balanced_braces() {
        let out = minify(SOURCE);
        assert_eq!(out.matches('{').count(), out.matches('}').count(), "{out}");
        assert!(!out.contains("/*"));
    }
}
