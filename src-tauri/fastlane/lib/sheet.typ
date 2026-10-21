#import "palette.typ": *

// Tailwind v4 ships its palette as oklch and typewind keeps it that way, so the
// sheet emits oklch rather than clipping through sRGB hex — `to-hex` dulls
// amber-400, which sits outside sRGB.
#let css-color(c) = {
  let (l, chroma, hue, _) = c.components()
  let n(x, digits) = str(calc.round(x, digits: digits))
  "oklch(" + n(l / 1%, 2) + "% " + n(chroma, 4) + " " + n(hue / 1deg, 2) + "deg)"
}

// The page and the assets read the same bindings, so the sheet cannot drift
// from what the PNGs are painted with.
#let sheet-palette = (
  bg: ink,
  surface: tw.slate-900,
  raised: tw.slate-800,
  edge: edge,
  muted: muted,
  ink: paper,
  todo: accent.at(2),
)

#let sheet-root = (
  (":root {", "  color-scheme: dark;")
    + sheet-palette.pairs().map(((name, c)) => "  --" + name + ": " + css-color(c) + ";")
    + ("}",)
).join("\n")

#let sheet-style = sheet-root + "\n" + ```css
  body {
    margin: 0 auto; padding: 40px 32px 72px; max-width: 1200px;
    background: var(--bg); color: var(--ink);
    font: 16px/1.5 "Albert Sans", system-ui, sans-serif;
  }
  h1 { font-size: 28px; font-weight: 700; margin: 0 0 4px; }
  h2 {
    font-size: 13px; font-weight: 600; letter-spacing: .08em;
    text-transform: uppercase; color: var(--muted);
    margin: 40px 0 16px; padding-bottom: 8px; border-bottom: 1px solid var(--edge);
  }
  .sheet { display: grid; gap: 20px; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); }
  .card { margin: 0; border: 1px solid var(--edge); border-radius: 12px; overflow: hidden; }
  .card a { display: block; line-height: 0; }
  /* Chequerboard reads through transparency, so a stray alpha channel shows. */
  .card img {
    width: 100%; height: 260px; object-fit: contain; padding: 12px; box-sizing: border-box;
    background-color: var(--surface);
    background-image:
      linear-gradient(45deg, var(--raised) 25%, transparent 25%, transparent 75%, var(--raised) 75%),
      linear-gradient(45deg, var(--raised) 25%, transparent 25%, transparent 75%, var(--raised) 75%);
    background-size: 16px 16px; background-position: 0 0, 8px 8px;
  }
  figcaption { padding: 12px 14px; border-top: 1px solid var(--edge); }
  figcaption code { font-family: "Monaspace Argon", ui-monospace, monospace; font-size: 13px; }
  figcaption .spec { display: block; margin-top: 4px; font-size: 12px; color: var(--muted); }
  .todo { color: var(--todo); }
```.text

// The bundle's own index: every asset the same list renders, linked and
// previewed, with the store's requirement beside it to eyeball against.
//
// Previews are raw `<img>` rather than `image(..)`, which inlines its source as
// a base64 data URI — a dozen screenshots would make a megabyte of markup and
// would show the source file rather than the rendered asset.
#let contact-sheet(assets) = {
  title()
  html.elem("style", sheet-style)
  // Listed in the order the array declares, which is the order the slots
  // appear in the store console.
  let groups = ()
  for a in assets {
    if a.group not in groups { groups.push(a.group) }
  }
  for name in groups {
    html.elem("h2", name)
    html.elem("div", attrs: (class: "sheet"), {
      for a in assets.filter(a => a.group == name) {
        html.elem("figure", attrs: (class: "card"), {
          html.elem(
            "a",
            attrs: (href: a.path),
            html.elem("img", attrs: (src: a.path, alt: a.path, loading: "lazy")),
          )
          html.elem("figcaption", {
            html.elem("code", a.path)
            html.elem(
              "span",
              attrs: (class: if a.at("todo", default: false) { "spec todo" } else { "spec" }),
              if a.at("todo", default: false) { "placeholder · " + a.spec } else { a.spec },
            )
          })
        })
      }
    })
  }
}
