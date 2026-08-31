//! Finding ```` ```lini ```` blocks in a chapter's markdown.

use std::ops::Range;

/// The info string that marks a block as ours.
const LANG: &str = "lini";

/// Rewrite every lini block in `markdown` through `render`.
///
/// `render` is handed the block's source — the block's own indent stripped off
/// each line, so a fence nested in a list still yields valid lini — the 1-based
/// line that source starts on, and the words the info string carries after the
/// language (`` ```lini figure ``). What it returns replaces the fence.
///
/// Every fenced block is walked, not just ours, so a ```` ```lini ```` shown as
/// an example inside a wider fence passes through as the text it is.
pub fn rewrite(markdown: &str, mut render: impl FnMut(&str, usize, &[&str]) -> String) -> String {
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut out = String::with_capacity(markdown.len());
    let mut i = 0;

    while i < lines.len() {
        let Some(fence) = fence_at(&lines, i) else {
            push_line(&mut out, &lines, i);
            i += 1;
            continue;
        };
        if fence.is_lini && fence.closed {
            // Blank lines lift the figure clear of any paragraph the
            // surrounding markdown would otherwise pull it into.
            let source = dedent(&lines[fence.content.clone()], fence.indent);
            out.push_str("\n\n");
            out.push_str(fence.indent);
            out.push_str(&render(&source, fence.content.start + 1, &fence.words));
            out.push_str("\n\n");
        } else {
            for j in i..=fence.end {
                push_line(&mut out, &lines, j);
            }
        }
        i = fence.end + 1;
    }
    out
}

/// A fenced code block, whatever its language.
struct Fence<'a> {
    /// The leading whitespace of the opening fence line.
    indent: &'a str,
    /// Whether the info string's language is ours.
    is_lini: bool,
    /// The info string's words after the language — `` ```lini figure ``.
    words: Vec<&'a str>,
    /// The block's content, exclusive of both fence lines.
    content: Range<usize>,
    /// The block's last line: its closing fence, or the document's last line
    /// when the fence never closes.
    end: usize,
    closed: bool,
}

/// Read the fenced block opening at `start`, or `None` if none opens there.
fn fence_at<'a>(lines: &[&'a str], start: usize) -> Option<Fence<'a>> {
    let (indent, rest) = split_indent(lines[start]);
    let marker = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let width = rest.chars().take_while(|c| *c == marker).count();
    if width < 3 {
        return None;
    }
    // A backtick fence's info string may not itself hold a backtick.
    let info = rest[width..].trim();
    if marker == '`' && info.contains('`') {
        return None;
    }

    let (lang, words) = split_info(info);
    let close = (start + 1..lines.len()).find(|&j| closes(lines[j], indent, marker, width));
    Some(Fence {
        indent,
        is_lini: lang == Some(LANG),
        words,
        content: start + 1..close.unwrap_or(lines.len()),
        end: close.unwrap_or(lines.len() - 1),
        closed: close.is_some(),
    })
}

/// Split an info string into its language and the words that follow it.
///
/// Whitespace and commas both separate, because mdbook's own fences take
/// commas — ```` ```rust,ignore ```` — so a reader who writes
/// ```` ```lini,figure ```` means what they appear to mean.
fn split_info(info: &str) -> (Option<&str>, Vec<&str>) {
    let mut parts = info.split([' ', '\t', ',']).filter(|p| !p.is_empty());
    (parts.next(), parts.collect())
}

/// A closing fence carries the opening indent, then at least as many of the
/// opening marker and nothing else.
fn closes(line: &str, indent: &str, marker: char, width: usize) -> bool {
    let Some(rest) = line.strip_prefix(indent) else {
        return false;
    };
    let run = rest.chars().take_while(|c| *c == marker).count();
    run >= width && rest[run..].trim().is_empty()
}

fn split_indent(line: &str) -> (&str, &str) {
    line.split_at(line.len() - line.trim_start_matches([' ', '\t']).len())
}

fn dedent(lines: &[&str], indent: &str) -> String {
    lines
        .iter()
        .map(|line| line.strip_prefix(indent).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn push_line(out: &mut String, lines: &[&str], i: usize) {
    out.push_str(lines[i]);
    if i + 1 < lines.len() {
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect what `rewrite` hands the renderer, and stub the output.
    fn calls(markdown: &str) -> (Vec<(String, usize)>, String) {
        let mut seen = Vec::new();
        let out = rewrite(markdown, |source, line, _words| {
            seen.push((source.to_owned(), line));
            "<svg/>".into()
        });
        (seen, out)
    }

    /// The words an info string yields, for the block it opens.
    fn words_of(markdown: &str) -> Vec<Vec<String>> {
        let mut seen = Vec::new();
        rewrite(markdown, |_source, _line, words| {
            seen.push(words.iter().map(|w| (*w).to_owned()).collect());
            "<svg/>".into()
        });
        seen
    }

    #[test]
    fn replaces_a_block_and_reports_its_first_line() {
        let (seen, out) = calls("intro\n\n```lini\na -> b\n```\n\ntail\n");
        assert_eq!(seen, [("a -> b".to_owned(), 4)]);
        assert!(out.contains("<svg/>"), "{out}");
        assert!(out.contains("intro") && out.contains("tail"));
    }

    #[test]
    fn strips_the_indent_of_a_nested_fence() {
        let (seen, _) = calls("1. step\n\n   ```lini\n   a -> b\n   ```\n");
        assert_eq!(seen, [("a -> b".to_owned(), 4)]);
    }

    #[test]
    fn leaves_other_languages_alone() {
        let markdown = "```rust\nfn main() {}\n```\n";
        let (seen, out) = calls(markdown);
        assert!(seen.is_empty());
        assert_eq!(out, markdown);
    }

    #[test]
    fn leaves_a_lini_block_quoted_inside_a_wider_fence_alone() {
        let markdown = "````markdown\n```lini\na -> b\n```\n````\n";
        let (seen, out) = calls(markdown);
        assert!(seen.is_empty());
        assert_eq!(out, markdown);
    }

    #[test]
    fn leaves_an_unclosed_block_alone() {
        let markdown = "```lini\na -> b\n";
        let (seen, out) = calls(markdown);
        assert!(seen.is_empty());
        assert_eq!(out, markdown);
    }

    #[test]
    fn renders_two_blocks_in_one_chapter() {
        let (seen, _) = calls("```lini\na\n```\n\ntext\n\n```lini\nb\n```\n");
        assert_eq!(seen, [("a".to_owned(), 2), ("b".to_owned(), 8)]);
    }

    #[test]
    fn a_bare_fence_carries_no_words() {
        assert_eq!(words_of("```lini\na\n```\n"), [Vec::<String>::new()]);
    }

    #[test]
    fn a_word_after_the_language_is_reported() {
        assert_eq!(words_of("```lini figure\na\n```\n"), [["figure"]]);
    }

    /// mdbook's own fences take commas — ```` ```rust,ignore ```` — so a reader
    /// who writes ```` ```lini,figure ```` means the same thing.
    #[test]
    fn commas_separate_words_as_whitespace_does() {
        assert_eq!(words_of("```lini,figure\na\n```\n"), [["figure"]]);
        assert_eq!(words_of("```lini,  figure ,\na\n```\n"), [["figure"]]);
    }

    #[test]
    fn several_words_are_all_reported() {
        assert_eq!(words_of("```lini figure other\na\n```\n"), [["figure", "other"]]);
    }

    /// The language must be the whole first word: a fence for some other
    /// language whose name merely starts with ours is not ours.
    #[test]
    fn a_longer_language_name_is_not_ours() {
        let (seen, out) = calls("```linigraph\na -> b\n```\n");
        assert!(seen.is_empty(), "{seen:?}");
        assert!(out.contains("linigraph"));
    }
}
