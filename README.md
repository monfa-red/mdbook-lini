<p align="center">
  <img src="https://raw.githubusercontent.com/monfa-red/mdbook-lini/main/assets/logo/lini_icon.svg" alt="Lini" width="128">
</p>

<p align="center"><strong>Diagrams in your mdbook, from plain text.</strong></p>

<p align="center">
  <a href="https://crates.io/crates/mdbook-lini"><img src="https://img.shields.io/crates/v/mdbook-lini.svg" alt="crates.io"></a>
  <a href="https://github.com/monfa-red/mdbook-lini/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="license: MIT"></a>
</p>

An [mdbook](https://rust-lang.github.io/mdBook/) preprocessor that compiles ` ```lini `
blocks to inline SVG at build time.

````markdown
```lini
cat -> dog -> bird
```
````

[Lini](https://lini.rs) is one engine for every figure family — flowcharts, charts,
sequences, mindmaps, trees, schematics, and technical drawings are all layouts of the same
language. So one fence covers all of them, and there is nothing else to install.

- **No runtime.** Figures are SVG in the HTML. No JavaScript, no CDN, no browser at build
  time — the pages work with scripts off.
- **No second binary.** Lini is linked as a library. `cargo install mdbook-lini` is the
  whole toolchain.
- **Dark mode for free.** Colours are live CSS variables, so figures follow your theme
  toggle without a re-render.
- **Fast and deterministic.** A typical diagram compiles in ~2 ms, byte-identically each
  run.

## Install

```bash
cargo install mdbook-lini
```

## Setup

Add the preprocessor to `book.toml`. mdbook finds the `mdbook-lini` binary from the key,
so there is nothing else to point at:

```toml
[preprocessor.lini]
```

Then copy [`mdbook-lini.css`](mdbook-lini.css) next to your `book.toml` and register it:

```toml
[output.html]
additional-css = ["mdbook-lini.css"]
```

That stylesheet is what sizes a figure and binds it to your theme. Without it the SVGs
still render, but they will not resize with the column or follow the light/dark toggle.

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

## Theming

Each figure is a `<div class="lini-figure">` wrapping the SVG. Lini emits every colour as
a `light-dark()` pair keyed on `color-scheme`, and the stylesheet binds that to mdbook's
themes — which is the entire light/dark integration.

To hand Lini your own palette, alias its role variables. Lini's defaults sit in
`@layer lini.defaults`, so any unlayered rule wins without `!important`:

```css
.lini {
    --lini-bg: transparent;
    --lini-fg: var(--fg);
    --lini-accent: #4a7fd4;
    --lini-font-family: var(--mono-font);
}
```

Its eleven-hue palette (`--rose`, `--sky`, `--teal`, … each in five tiers) is emitted only
where a figure references it.

> [!NOTE]
> Set `--lini-font-family` only to a font metrically identical to the one Lini measured
> against at compile time. Lini bakes text positions at build time, so a font with
> different metrics will drift.

## Sizing

The wrapper carries `--lini-w`, the diagram's natural width. A figure scales down to fit
the column but stops at 75% of that width — past there the labels stop reading, so the
wrapper scrolls horizontally instead.

## Errors

A block that fails to compile becomes a visible `<pre class="lini-error">` on the page and
a message on stderr; the rest of the build carries on, so one bad diagram never costs you
the book. Warnings — an unroutable link, say — only go to stderr. Diagnostics carry the
chapter path and the real line number in the markdown file:

```
mdbook-lini: figures.md:41:1: warning: impossible (a -> b): no legal route
```

## Links

- [lini.rs](https://lini.rs) — language reference and gallery
- [github.com/monfa-red/lini](https://github.com/monfa-red/lini) — the Lini compiler

## License

MIT
