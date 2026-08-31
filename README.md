<p align="center">
  <img src="https://raw.githubusercontent.com/monfa-red/mdbook-lini/main/assets/logo/lini_icon.svg" alt="Lini" width="128">
</p>

<p align="center"><strong>Every figure in your mdbook, from plain text.</strong></p>

<p align="center">
  <a href="https://crates.io/crates/mdbook-lini"><img src="https://img.shields.io/crates/v/mdbook-lini.svg" alt="crates.io"></a>
  <a href="https://github.com/monfa-red/mdbook-lini/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="license: MIT"></a>
</p>

An [mdbook](https://rust-lang.github.io/mdBook/) preprocessor that compiles ` ```lini `
blocks to inline SVG at build time.

````markdown
```lini
|chart| "Signups by channel" { categories: "Jan", "Feb", "Mar", "Apr", "May", "Jun" } [
  |line| "organic"  { data: 14, 19, 26, 33, 44, 58; curve: smooth; marker: dot; stroke: --teal }
  |line| "referral" { data: 9, 13, 15, 22, 27, 36; curve: smooth; marker: dot; stroke: --purple }
]
```
````

That is a chart, but the fence is not a chart fence. [Lini](https://lini.rs) is one engine
for every figure family — flowcharts, charts, sequences, mindmaps, trees, schematics, and
technical drawings are all layouts of the same language. So one fence covers all of them,
and there is nothing else to install.

- **No runtime.** Figures are SVG in the HTML. No JavaScript, no CDN, no browser at build
  time — the pages work with scripts off.
- **No second binary, no setup.** Lini is linked as a library, and the styling ships with
  the figures. `cargo install mdbook-lini` plus one line of config is the whole thing.
- **Dark mode for free.** Colours are live CSS variables, so figures follow your theme
  toggle without a re-render.
- **The source is one click away.** Each figure carries a small `</>` toggle that reveals
  the Lini that drew it, syntax-highlighted at build time — still no JavaScript. A fence
  word flips it, so a reference chapter can lead with the source instead.
- **Fast and deterministic.** A typical diagram compiles in ~2 ms, byte-identically each
  run.

## Install

```bash
cargo install mdbook-lini
```

## Setup

One line in `book.toml`. mdbook finds the `mdbook-lini` binary from the key, and the
styling ships with the figures:

```toml
[preprocessor.lini]
```

That's it — no `additional-css`, no files to copy.

## Writing a figure

Any Lini source works. The [language reference](https://lini.rs) covers it in full; this
is the shape of it:

````markdown
```lini
{ layout: sequence; font-size: 13; }

|icon#reader| "user" { width: 60; stroke: --rose-deep; fill: --rose-wash }
|box#cdn| "CDN" { fill: --sky-wash; stroke: --sky-deep }
|box#origin| "Origin" { fill: --green-wash; stroke: --green-deep }

reader -> cdn "GET /guide"
cdn -> cdn "check edge cache"

|loop| "on miss" { fill: --amber-wash; stroke: --amber-ink } [
  cdn -> origin "fetch"
  origin --> cdn "200 + max-age"
]

cdn --> reader "HTML"
```
````

A block that references a local image with `|image| src:` resolves the path against the
chapter's own directory.

Blocks in other languages pass through untouched, as does a ` ```lini ` block quoted
inside a wider fence — so you can document Lini in a Lini-powered book.

## Showing the source

Every figure carries a small `</>` in its top-right corner. Clicking it reveals the Lini
that drew it, highlighted with the same vocabulary the VS Code and Zed grammars use — the
colouring comes from Lini's own ledger, so a property gets its colour here the moment the
language has it.

The toggle is a checkbox and its label — pure CSS, so it costs no JavaScript, takes
keyboard focus, and works with scripts off. The listing is your block's own text,
verbatim, not reformatted; mdbook's copy button lands on it like any other code block.

It is deliberately not a `<details>`. Your book's stylesheet is unlayered, so it outranks
ours: a `<details>` gives your theme a second element to frame — a box inside the code
block's box — and it carries a disclosure marker your theme can put back however we hide
it. With a label there is no marker, and the `<pre>` is the only element your theme
dresses. One frame, like every other code block on the page.

### Choosing what a block shows

One word on the fence, and each word means exactly one thing:

| fence | shows | the toggle reveals |
| --- | --- | --- |
| ` ```lini ` | the figure | the source |
| ` ```lini code ` | the source | the figure |
| ` ```lini figure ` | the figure alone | — |
| ` ```lini raw ` | the source alone | — |

`code` is the one to reach for in a chapter that teaches syntax — the source is the
lesson, and the figure is a click away rather than the other way round.

`raw` is the odd one: it never reaches the compiler. That is the point of it. A fragment,
a counter-example, or a deliberately broken line stays a highlighted listing instead of
becoming an error box, so you can write about Lini that isn't meant to draw:

````markdown
```lini raw
|box#hero| "…"   // a shape, not a whole file
```
````

Whitespace or a comma both separate, so ` ```lini,figure ` reads the same. The three words
are alternatives — name two and the last wins. A word we don't recognise is reported on
stderr and ignored, never fatal.

## Theming

Each figure is a `<div class="lini-figure">` wrapping the SVG. Lini emits every colour as
a `light-dark()` pair keyed on `color-scheme`, and the shipped styling binds that to
mdbook's five built-in themes — which is the entire light/dark integration. A custom theme
adds its own class:

```css
html.my-dark-theme .lini { color-scheme: dark; }
```

To hand Lini your own palette, alias its role variables. Everything shipped sits in
`@layer`, so any unlayered rule of yours wins without `!important`:

```css
.lini {
    --lini-bg: transparent;
    --lini-fg: var(--fg);
    --lini-accent: #4a7fd4;
    --lini-font-family: var(--body-font);
}
```

Its eleven-hue palette (`--rose`, `--sky`, `--teal`, … each in five tiers) is emitted only
where a figure references it.

> [!NOTE]
> Alias `--lini-font-family` only to a proportional sans close in metrics to the one Lini
> measured against at compile time. Lini bakes each label's position and sizes its box to
> fit, so a wider face — a monospace one especially — pushes the text past its border.

## Sizing

The wrapper carries `--lini-w`, the diagram's natural width. A figure scales down to fit
the column but stops at 75% of that width — past there the labels stop reading, so the
wrapper scrolls horizontally instead.

The source listing is not bound by that floor: it fills the column and scrolls
horizontally on its own when a line is long.

## Owning the styling

mdbook-lini's own stylesheet — the wrapper above, the theme binding, the error box, and
the source listing's palette — rides along in a `<style>` block on each chapter that has a
figure. Under 3 kB minified, and none on chapters without one. It is not Lini's styling:
that lives inside each SVG and travels with it regardless.

To take it over instead, turn it off and link [`mdbook-lini.css`](mdbook-lini.css)
yourself:

```toml
[preprocessor.lini]
bundled-css = false

[output.html]
additional-css = ["mdbook-lini.css"]
```

## Errors

A block that fails to compile becomes a visible `<pre class="lini-error">` on the page and
a message on stderr; the rest of the build carries on, so one bad diagram never costs you
the book. Warnings — an unroutable link, say — only go to stderr. Diagnostics carry the
chapter path and the real line number in the markdown file:

```
mdbook-lini: figures.md:41:1: warning: impossible (a -> b): no legal route
```

## Links

Everything about the language itself lives with Lini, not here:

- [lini.rs](https://lini.rs) — language reference and gallery
- [github.com/monfa-red/lini](https://github.com/monfa-red/lini) — the compiler, its `SPEC.md`,
  and `samples/` for every figure family

## License

MIT
