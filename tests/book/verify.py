#!/usr/bin/env python3
"""Check that mdbook accepted the HTML the preprocessor emitted.

The unit tests check the markup we produce. This checks the thing they
cannot see: whether mdbook's Markdown parser kept it in one piece.

The failure it guards is silent and total. A blank line ends an HTML block
in CommonMark, so a listing carrying one would spill the rest of the chapter
out of the figure and strand the closing tags at the foot of the page —
with no error from anything, and a book that still builds.

Usage: verify.py <built chapter.html>
"""

import re
import sys


def main(path: str) -> int:
    html = open(path, encoding="utf-8").read()
    body = re.search(r"<main>(.*?)</main>", html, re.S)
    if not body:
        print(f"FAIL: no <main> in {path}")
        return 1
    # Our stylesheet rides along inside the chapter and names every class we
    # look for below, so it has to go before anything is counted.
    body = re.sub(r"<style>.*?</style>", "", body.group(1), flags=re.S)

    failures = []

    def want(condition: bool, what: str) -> None:
        if not condition:
            failures.append(what)

    # Six lini blocks: two plain, one `figure`, one `code`, one `raw`, and one
    # that does not compile.
    want(body.count('<div class="lini-figure-block">') == 3, "expected 3 figure blocks")
    want(body.count('<div class="lini-source">') == 4, "expected 4 listings")
    want(body.count('class="lini-view-toggle"') == 3, "expected 3 toggles")
    want(body.count('<div class="lini-figure"') == 4, "expected 4 figures")

    # `code` puts the listing first and the figure behind the toggle. Found by
    # its label rather than by position, so reordering the fixture is harmless.
    blocks = body.split('<div class="lini-figure-block">')[1:]
    code_blocks = [b for b in blocks if "Show figure" in b]
    want(len(code_blocks) == 1, f"expected 1 `code` block, found {len(code_blocks)}")
    for block in code_blocks:
        want(
            block.find("lini-source") < block.find('lini-figure"'),
            "the `code` block still leads with its figure",
        )
        want(
            block.find("lini-alt-view") < block.find('lini-figure"'),
            "the figure is not the hidden half in `code` mode",
        )

    # In `code` mode the control is inside mdbook's own button row, so mdbook
    # lays both out and the pointer never leaves the <pre> on its way to ours.
    # Placed on top instead, it drifts out of alignment and steals the hover
    # that keeps mdbook's copy button on screen.
    want(
        '<pre><span class="buttons"><label class="lini-view-button"' in body,
        "the `code` control is not in mdbook's button row",
    )
    # The default mode has no <pre> on top to join, so it must not emit a row.
    want(body.count('class="buttons"') == 1, "expected exactly 1 button row")

    # `raw` is a listing and nothing else — no figure, no toggle, no error box
    # even though the fragment does not compile.
    want(body.count("a shape, not a whole file") == 1, "the raw fragment is missing")

    # A <details> brings a disclosure marker a host stylesheet can re-assert
    # from outside our layer, and a second element for a theme to frame. The
    # caret and the box-in-a-box on lini.rs were both that element.
    want("<details" not in body, "a <details> crept back in")
    want("<summary" not in body, "a <summary> crept back in")

    # Every toggle id is unique and every label points at its own.
    ids = re.findall(r'class="lini-view-toggle" type="checkbox" id="([^"]+)"', body)
    want(len(ids) == len(set(ids)), f"duplicate toggle ids: {ids}")
    for one in ids:
        want(f'for="{one}"' in body, f"no label points at {one}")
    want('class="nohighlight"' in body, "listing does not opt out of hljs")
    want('class="lini-tok-' in body, "listing is not highlighted")

    # A block that failed to compile is on the page, and did not take the
    # page with it.
    want(body.count('<pre class="lini-error">') == 1, "expected 1 error box")

    # Every paragraph survived, in order.
    prose = [
        "Prose after the chart, which must not be swallowed.",
        "One opted out with",
        "The end.",
    ]
    at = -1
    for text in prose:
        found = body.find(text, at + 1)
        want(found > at, f"prose out of order or missing: {text!r}")
        at = found

    # The signature of a shattered HTML block: closing tags stranded after the
    # last prose, and paragraphs that were never meant to be paragraphs.
    tail = body[body.rfind("The end.") :]
    want("</pre>" not in tail, f"orphaned </pre> after the last prose: {tail[:200]!r}")
    want("</details>" not in tail, f"orphaned </details>: {tail[:200]!r}")

    # The source we emitted carries `&#10;` rather than newlines, so the block
    # scanner never sees a blank line. mdbook decodes those entities on the way
    # out — by then the block's extent is settled — so the reader gets the
    # author's own line breaks back. Both halves of that have to hold: the
    # listing shows the blank lines, and the chapter above stayed whole.
    listings = re.findall(r"<code class=\"nohighlight\">(.*?)</code>", body, re.S)
    want(len(listings) == 4, f"expected 4 listings, found {len(listings)}")
    spaced = [text for text in listings if "\n\n" in re.sub(r"<[^>]*>", "", text)]
    want(len(spaced) == 1, "the blank-line source did not keep its blank lines")

    if failures:
        for f in failures:
            print(f"FAIL: {f}")
        return 1
    print(f"{path}: chapter intact")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
