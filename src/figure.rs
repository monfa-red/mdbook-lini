//! Compiling one lini block into the figure that replaces it.

use std::path::Path;

use lini::{Diagnostic, Level, Options};

/// The wrapper a figure and its listing share. It must stay outside Lini's own
/// `.lini-<type>` namespace — every node in a diagram wears `.lini-box`,
/// `.lini-block` and the like, so a wrapper named for a type would restyle the
/// insides of every SVG on the page. `our_classes_are_ours_alone` guards it.
const BLOCK: &str = "lini-figure-block";

/// Open a drawn icon of `width` — the two marks below share everything but
/// their strokes, and neither may carry a newline.
const ICON_HEAD: &str = concat!(
    r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" "#,
    r#"stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">"#,
);

/// "Show the source": a pair of chevrons with air between them. A slash in the
/// middle only crowds it at this size.
const ICON_SOURCE: &str =
    r#"<path d="M9 6.5 3.5 12 9 17.5"/><path d="M15 6.5 20.5 12 15 17.5"/></svg>"#;

/// "Show the figure": a framed picture. Deliberately not a play triangle —
/// nothing runs here, the figure is already drawn and merely hidden, and a play
/// glyph would promise otherwise. Two linked nodes would say `lini` more
/// loudly, but at 18px the boxes collapse into a pair of dots.
const ICON_FIGURE: &str = concat!(
    r#"<rect x="3" y="4.75" width="18" height="14.5" rx="2.5"/>"#,
    r#"<circle cx="8.75" cy="10" r="1.5"/>"#,
    r#"<path d="M20.5 17.5 14.25 11.25 5 19.25"/></svg>"#,
);

/// What a block shows, and in which order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    /// The default: the figure, with the source behind the toggle.
    FigureFirst,
    /// `code` — the source, with the figure behind the toggle.
    CodeFirst,
    /// `figure` — the figure alone, no toggle and no listing.
    FigureOnly,
    /// `raw` — the listing alone. Never compiled, so a fragment or a
    /// deliberate counter-example stays a listing instead of an error box.
    RawOnly,
}

/// Compile one block's source into its figure, and — unless the fence says
/// `figure` — the collapsed listing of the source beside it.
///
/// A block that fails to compile becomes a visible error box and the message
/// goes to stderr — the build still finishes, so one bad diagram never costs
/// you the rest of the book.
pub fn render(
    source: &str,
    chapter: &str,
    first_line: usize,
    base_dir: Option<&Path>,
    words: &[&str],
) -> String {
    let mode = mode(words, chapter, first_line);

    // `raw` never reaches the compiler, which is the whole point of it: a
    // fragment, a counter-example, or a deliberate syntax error is a listing
    // to read, not a figure that failed to draw.
    if mode == Mode::RawOnly {
        return listing(source, "");
    }

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
            let id = toggle_id(chapter, first_line);
            match mode {
                Mode::FigureOnly => wrap(&svg),
                // Figure first: the control floats over the figure, which has
                // no button row of its own to join.
                Mode::FigureFirst => format!(
                    "<div class=\"{BLOCK}\">{figure}{toggle}{button}\
                     <div class=\"lini-alt-view\">{alt}</div></div>",
                    figure = wrap(&svg),
                    toggle = checkbox("source", &id),
                    button = button(ICON_SOURCE, "source", &id),
                    alt = listing(source, ""),
                ),
                // Source first: the control joins mdbook's own button row
                // inside the `<pre>` — see `listing`.
                Mode::CodeFirst => format!(
                    "<div class=\"{BLOCK}\">{toggle}{code}\
                     <div class=\"lini-alt-view\">{alt}</div></div>",
                    toggle = checkbox("figure", &id),
                    code = listing(
                        source,
                        &format!(
                            "<span class=\"buttons\">{}</span>",
                            button(ICON_FIGURE, "figure", &id)
                        ),
                    ),
                    alt = wrap(&svg),
                ),
                Mode::RawOnly => unreachable!("returned above"),
            }
        }
        Err(e) => {
            let text = e.display_with_source(&padded, chapter).to_string();
            eprintln!("mdbook-lini: {text}");
            error_box(&text)
        }
    }
}

/// Read the mode off the fence's words.
///
/// The three words are alternatives, so the last one named wins. A word we
/// don't know is reported and ignored, not fatal — the same bargain the rest of
/// this module strikes: a typo in an info string costs a line of build output,
/// never the figure.
fn mode(words: &[&str], chapter: &str, line: usize) -> Mode {
    let mut mode = Mode::FigureFirst;
    for word in words {
        mode = match *word {
            "figure" => Mode::FigureOnly,
            "code" => Mode::CodeFirst,
            "raw" => Mode::RawOnly,
            other => {
                eprintln!(
                    "mdbook-lini: {chapter}:{line}: unknown word `{other}` on a lini fence — ignoring"
                );
                mode
            }
        };
    }
    mode
}

/// A figure and the collapsed listing of the source that drew it.
///
/// A checkbox and its label are the whole mechanism — no script, so the toggle
/// works with scripts off and takes keyboard focus, which is the promise the
/// figures themselves make.
///
/// It is deliberately **not** a `<details>`. That element carries a disclosure
/// marker, and a book's own stylesheet is unlayered — so it outranks ours and
/// can put the caret back however we suppress it. It also gives a theme a
/// second element to frame, which is how a listing ends up in a box inside a
/// box. A label has no marker, and the `<pre>` is left as the one thing a theme
/// will style: one frame, like every other code block on the page.
fn checkbox(names: &str, id: &str) -> String {
    format!(
        "<input class=\"lini-view-toggle\" type=\"checkbox\" id=\"{id}\" aria-label=\"Show {names}\">"
    )
}

fn button(icon: &str, names: &str, id: &str) -> String {
    format!(
        "<label class=\"lini-view-button\" for=\"{id}\" title=\"Show {names}\">{ICON_HEAD}{icon}</label>"
    )
}

/// The source, highlighted, in the panel the token palette is keyed on.
///
/// The `<pre>` inside is left bare so it is the one element a book's own
/// stylesheet frames — one box, like every other code block on the page.
/// `nohighlight` makes mdbook's highlight.js bail before it re-tokenizes our
/// spans, while its script still adds `.hljs` for the theme's code chrome and
/// still hangs its copy button on `pre > code`.
/// `buttons` is mdbook's own control row, emitted by us when the listing is
/// the primary view. mdbook's script adopts any `.buttons` it finds inside a
/// `<pre>` and inserts its copy button there rather than building its own — so
/// mdbook positions both controls, they cannot drift out of alignment, and the
/// pointer never leaves the `<pre>` on its way to ours, which is what kept the
/// copy button from vanishing under the hand-placed version.
fn listing(source: &str, buttons: &str) -> String {
    format!(
        "<div class=\"lini-source\"><pre>{buttons}<code class=\"nohighlight\">{}</code></pre></div>",
        one_line(&lini::highlight_html(source)),
    )
}

/// A DOM id for one block's toggle, unique within the chapter — which is the
/// page. Anything that is not a letter or digit folds to `-`, so a nested
/// chapter path still yields a legal id and the label can point at it.
fn toggle_id(chapter: &str, line: usize) -> String {
    let mut slug = String::with_capacity(chapter.len());
    for c in chapter.chars() {
        match c {
            c if c.is_ascii_alphanumeric() => slug.push(c.to_ascii_lowercase()),
            _ if !slug.ends_with('-') => slug.push('-'),
            _ => {}
        }
    }
    format!("lini-src-{}-{line}", slug.trim_matches('-'))
}

/// Fold every newline into the entity that renders as one.
///
/// A blank line **ends an HTML block** in CommonMark: the rest of the chapter
/// would be parsed as Markdown and spill out of the figure, orphaning the
/// closing tags at the foot of the page. Idiomatic Lini has blank lines — a
/// stylesheet, a gap, then the drawn statements — so the listing cannot carry
/// them literally. Inside `<pre>` the entity renders identically, and it can
/// end nothing.
fn one_line(html: &str) -> String {
    html.replace('\n', "&#10;")
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

/// A diagnostic, on the page as well as on stderr. It is many lines and may
/// carry an empty one, so it goes through [`one_line`] like the listing does.
fn error_box(message: &str) -> String {
    format!("<pre class=\"lini-error\">{}</pre>", one_line(&escape(message)))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The width hook rides the figure div in either mode — the listing is
    /// beside the figure, never between it and its own sizing.
    #[test]
    fn wraps_a_diagram_with_its_natural_width() {
        for words in [&[][..], &["figure"][..]] {
            let html = render("a -> b", "demo.md", 1, None, words);
            assert!(
                html.contains("<div class=\"lini-figure\" style=\"--lini-w: "),
                "{words:?}: {html}"
            );
            assert!(html.contains("<svg") && html.ends_with("</div>"), "{words:?}: {html}");
        }
    }

    #[test]
    fn a_broken_block_becomes_an_error_box() {
        let html = render("|box", "demo.md", 1, None, &[]);
        assert!(html.starts_with("<pre class=\"lini-error\">"), "{html}");
    }

    #[test]
    fn an_error_points_at_the_chapter_line_not_the_block_line() {
        let html = render("a -> b\n|box", "demo.md", 41, None, &[]);
        assert!(html.contains("demo.md:42:"), "{html}");
    }

    #[test]
    fn reads_the_width_off_the_root_tag() {
        assert_eq!(natural_width(r#"<svg viewBox="0 0 8 4" width="8" height="4">"#), Some("8"));
        assert_eq!(natural_width("<svg>"), None);
    }

    /// A source that exercises the hazard: a blank line between the stylesheet
    /// and the drawn statements, which is how idiomatic lini is laid out.
    const SPACED: &str = "{ layout: flow; }\n\n|box#a| \"A\"\n\na -> b \"go\"\n";

    /// The tags this module emits, undone — enough to read back the listing.
    fn listing(html: &str) -> String {
        let open = html.find("<code").expect("a code element");
        let start = html[open..].find('>').expect("its end") + open + 1;
        let end = html[start..].find("</code>").expect("its close") + start;
        let mut out = String::new();
        let mut rest = &html[start..end];
        while let Some(lt) = rest.find('<') {
            out.push_str(&rest[..lt]);
            let gt = rest[lt..].find('>').expect("well-formed tag") + lt;
            rest = &rest[gt + 1..];
        }
        out.push_str(rest);
        out.replace("&#10;", "\n")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&amp;", "&")
    }

    /// A blank line ends an HTML block in CommonMark, spilling the rest of the
    /// chapter out of the figure — so nothing we emit may contain one.
    fn blank_line(html: &str) -> Option<usize> {
        html.split('\n').position(|line| line.trim().is_empty())
    }

    #[test]
    fn a_figure_carries_a_source_toggle_by_default() {
        let html = render("a -> b", "demo.md", 1, None, &[]);
        assert!(html.starts_with("<div class=\"lini-figure-block\">"), "{html}");
        assert!(html.contains("<input class=\"lini-view-toggle\""), "{html}");
        assert!(html.contains("<label class=\"lini-view-button\""), "{html}");
        assert!(html.contains("lini-source"), "{html}");
        assert!(html.contains("<div class=\"lini-figure\""), "{html}");
    }

    /// A `<details>` brings a disclosure marker that a host stylesheet can
    /// re-assert from outside our layer — the caret that turned up on
    /// lini.rs. A label has no marker to suppress in the first place.
    #[test]
    fn the_control_is_a_label_with_no_disclosure_marker() {
        let html = render("a -> b", "demo.md", 1, None, &[]);
        assert!(!html.contains("<details"), "{html}");
        assert!(!html.contains("<summary"), "{html}");
    }

    /// The listing sits in a plain `<div>`, so the `<pre>` inside it is the
    /// only element a host theme boxes — one frame, like every other code
    /// block on the page, rather than a box inside a box.
    #[test]
    fn only_the_pre_is_boxable() {
        let html = render("a -> b", "demo.md", 1, None, &[]);
        let panel = html.split("<div class=\"lini-source").nth(1).expect("a panel");
        let panel = &panel[panel.find('>').unwrap() + 1..];
        assert!(panel.starts_with("<pre><code"), "{panel}");
    }

    /// Two figures share a page, so their toggles cannot share an id — and the
    /// label has to point at its own.
    #[test]
    fn each_toggle_owns_its_id() {
        let first = render("a -> b", "guide/figures.md", 4, None, &[]);
        let second = render("a -> b", "guide/figures.md", 40, None, &[]);
        // Anchored on the toggle: a diagram's own nodes carry `data-id=` too.
        let id_of = |html: &str| {
            let mark = "<input class=\"lini-view-toggle\" type=\"checkbox\" id=\"";
            let at = html.find(mark).expect("a toggle") + mark.len();
            html[at..][..html[at..].find('"').unwrap()].to_owned()
        };
        let (a, b) = (id_of(&first), id_of(&second));
        assert_ne!(a, b, "two blocks in one chapter share an id");
        assert!(first.contains(&format!("for=\"{a}\"")), "label points elsewhere: {first}");
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'), "illegal id: {a}");
    }

    /// hljs re-tokenizes every `<code>` it finds and would wipe our spans; it
    /// bails early on this class alone (`noHighlightRe`), while mdbook still
    /// adds `.hljs` for the theme's code chrome and still attaches its copy
    /// button to `pre code`.
    #[test]
    fn the_listing_opts_out_of_mdbooks_highlighter() {
        let html = render("a -> b", "demo.md", 1, None, &[]);
        assert!(html.contains("<pre><code class=\"nohighlight\">"), "{html}");
    }

    #[test]
    fn the_figure_word_emits_the_bare_figure() {
        let html = render("a -> b", "demo.md", 1, None, &["figure"]);
        assert!(html.starts_with("<div class=\"lini-figure\" style=\"--lini-w: "), "{html}");
        assert!(!html.contains("lini-source"), "{html}");
        assert!(!html.contains("lini-figure-block"), "{html}");
        assert!(html.ends_with("</div>"));
    }

    #[test]
    fn an_unrecognised_word_still_renders_the_default() {
        let html = render("a -> b", "demo.md", 1, None, &["wat"]);
        assert!(html.contains("lini-source"), "{html}");
    }

    #[test]
    fn the_listing_is_the_authors_own_text_verbatim() {
        let html = render(SPACED, "demo.md", 1, None, &[]);
        assert_eq!(listing(&html), SPACED);
    }

    #[test]
    fn a_source_with_blank_lines_emits_no_blank_line() {
        let html = render(SPACED, "demo.md", 1, None, &[]);
        assert_eq!(blank_line(&html), None, "line {:?} is blank in {html}", blank_line(&html));
    }

    #[test]
    fn a_bare_figure_emits_no_blank_line() {
        let html = render(SPACED, "demo.md", 1, None, &["figure"]);
        assert_eq!(blank_line(&html), None, "{html}");
    }

    /// A diagnostic is many lines and may carry an empty one; it lands in the
    /// page as raw HTML like everything else, so it is under the same law.
    #[test]
    fn an_error_box_emits_no_blank_line() {
        let html = render("|box| { fill: ", "demo.md", 1, None, &[]);
        assert!(html.starts_with("<pre class=\"lini-error\">"), "{html}");
        assert_eq!(blank_line(&html), None, "{html}");
    }

    #[test]
    fn the_code_word_puts_the_source_first() {
        let html = render("a -> b", "demo.md", 1, None, &["code"]);
        let source_at = html.find("lini-source").expect("a listing");
        let figure_at = html.find("lini-figure\"").expect("a figure");
        assert!(source_at < figure_at, "the figure still comes first: {html}");
        assert!(html.contains("Show figure"), "{html}");
        assert!(html.contains("lini-alt-view"), "{html}");
    }

    /// The hidden half is whichever one the mode did not put first, so one
    /// rule hides either — the alt marker rides the figure in `code` mode.
    #[test]
    fn code_mode_hides_the_figure_not_the_listing() {
        let html = render("a -> b", "demo.md", 1, None, &["code"]);
        let alt = html.split("lini-alt-view").nth(1).expect("an alt panel");
        assert!(alt[..80].contains("lini-figure"), "the alt panel is not the figure: {alt}");
    }

    /// The control lives in mdbook's own row, so mdbook lays both out. A
    /// hand-placed button over the `<pre>` drifts out of alignment and, worse,
    /// steals the pointer off the `<pre>` so mdbook hides its copy button the
    /// moment you reach for ours.
    #[test]
    fn the_code_mode_button_joins_mdbooks_button_row() {
        let html = render("a -> b", "demo.md", 1, None, &["code"]);
        assert!(
            html.contains("<pre><span class=\"buttons\"><label class=\"lini-view-button\""),
            "{html}"
        );
    }

    /// Figure-first has no `<pre>` on top to join, so its control floats over
    /// the figure and must not carry a row that mdbook would style.
    #[test]
    fn the_default_mode_button_stands_alone() {
        let html = render("a -> b", "demo.md", 1, None, &[]);
        assert!(!html.contains("class=\"buttons\""), "{html}");
        assert!(html.contains("<label class=\"lini-view-button\""), "{html}");
    }

    #[test]
    fn the_raw_word_emits_a_listing_and_nothing_else() {
        let html = render("a -> b", "demo.md", 1, None, &["raw"]);
        assert!(html.contains("lini-source"), "{html}");
        assert!(!html.contains("<svg"), "raw drew a figure: {html}");
        assert!(!html.contains("lini-view-toggle"), "raw carries a toggle: {html}");
        assert!(!html.contains("lini-figure"), "{html}");
    }

    /// The point of `raw`: a fragment or a deliberate counter-example is a
    /// listing, not an error box. Nothing compiles it, so nothing can fail.
    #[test]
    fn a_raw_block_is_never_compiled() {
        let html = render("|box| { fill:", "demo.md", 1, None, &["raw"]);
        assert!(!html.contains("lini-error"), "raw reported a compile error: {html}");
        assert!(html.contains("lini-tok-"), "raw lost its highlighting: {html}");
    }

    #[test]
    fn a_raw_listing_emits_no_blank_line() {
        let html = render(SPACED, "demo.md", 1, None, &["raw"]);
        assert_eq!(blank_line(&html), None, "{html}");
    }

    /// Lini dresses every node in `.lini-<type>`, so a class of ours that
    /// happened to be spelled like a type would restyle the insides of every
    /// diagram on the page. This is the standing guard — it is what turns a
    /// future Lini release adding, say, a `figure` type from a silent visual
    /// bug into a failing build here.
    #[test]
    fn our_classes_are_ours_alone() {
        // One of each family, so the sweep sees the classes each one emits.
        let src = "{ layout: flow; }\n|box#a| \"A\"\n|cyl#b| \"B\"\n|note#c| \"C\"\na -> b";
        let svg = lini::compile_str(src).expect("the probe compiles");
        for ours in [
            BLOCK,
            "lini-figure",
            "lini-source",
            "lini-error",
            "lini-view-toggle",
            "lini-view-button",
            "lini-alt-view",
        ] {
            assert!(
                !svg.contains(ours),
                "Lini now emits `{ours}` itself — rename ours before it restyles diagrams"
            );
        }
    }
}
