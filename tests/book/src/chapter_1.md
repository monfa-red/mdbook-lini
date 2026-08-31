# Figures

A chart, whose source is one click away.

```lini
|chart| "Signups by channel" { categories: "Jan", "Feb", "Mar" } [
  |line| "organic" { data: 14, 19, 26; curve: smooth; marker: dot; stroke: --teal }
]
```

Prose after the chart, which must not be swallowed.

A source laid out the idiomatic way — a stylesheet, a blank line, then the drawn
statements. The blank lines are the hazard this fixture exists for.

```lini
{ layout: sequence; font-size: 13; }

|icon#reader| "user" { width: 60; stroke: --rose-deep; fill: --rose-wash }
|box#cdn| "CDN" { fill: --sky-wash; stroke: --sky-deep }

reader -> cdn "GET /guide"
cdn --> reader "HTML"
```

One opted out with `figure`, which renders as it always did.

```lini figure
|box#a| "just"
a -> b "a picture"
```

Source first, with the figure behind the toggle — how a reference chapter reads.

```lini code
|box#draft| "draft"
draft -> review -> publish
```

A fragment that does not compile on its own. `raw` never reaches the compiler, so
it stays a listing rather than becoming an error box.

```lini raw
|box#hero| "…"   // a shape, not a whole file
{ fill: --teal-wash; }
```

A block that does not compile, which must not cost us the rest of the page.

```lini
|box| { fill:
```

The end.
