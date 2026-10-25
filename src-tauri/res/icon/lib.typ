// Paints the authored logo and masks with a palette. Nothing here edits the
// artwork: colours arrive as a stylesheet, and the logo is spliced into the
// mask's `#inner` as a tree. Paths are absolute from the app directory.

#import "@preview/xmlit:0.1.3": xml-to-string

// --- trees -------------------------------------------------------------------

#let load(path) = xml(path).filter(n => type(n) == dictionary).first()
#let id-of(node) = if type(node) == dictionary { node.attrs.at("id", default: none) }

#let ids(node) = {
  if type(node) != dictionary { return () }
  let own = if id-of(node) != none { (id-of(node),) } else { () }
  own + node.children.map(ids).sum(default: ())
}

#let replace-id(node, id, f) = {
  if type(node) != dictionary { return node }
  if id-of(node) == id { return f(node) }
  node + (children: node.children.map(c => replace-id(c, id, f)))
}

// --- artwork -----------------------------------------------------------------

// The connectorless master stays legible at small sizes.
#let masters = (xs: load("/res/logo/xs.svg"), md: load("/res/logo/md.svg"))
#let master(px) = if px <= 64 { "xs" } else { "md" }
#let logo-ids = masters.values().map(ids).sum()

// --- stylesheet --------------------------------------------------------------

// `[id=…]` because `SW→NW` is not a valid `#` selector.
#let stylesheet(rules) = (
  rules.map(((id, props)) => "[id=\"" + id + "\"]{" + props.pairs().map(((k, v)) => k + ":" + v).join(";") + "}").join()
)

#let require(rules, present, where) = {
  for (id, _) in rules {
    if id not in present {
      panic("palette entry `" + id + "` targets element #" + id + ", which is absent from " + where)
    }
  }
}

#let fill-rules(fills) = fills.pairs().map(((id, c)) => (id, (fill: c.to-hex(), stroke: c.to-hex())))

// In the root's namespace: a namespace-less node is serialized with `xmlns=""`,
// which takes the stylesheet out of SVG and the renderer ignores it.
#let styled(root, rules) = {
  let style = (namespace: root.namespace, tag: "style", attrs: (:), children: (stylesheet(rules),))
  root + (children: (style,) + root.children)
}

// --- rendering ---------------------------------------------------------------

// One SVG: `mask` with the palette's logo spliced into `#inner`.
// `background: none` leaves the mask's background transparent.
#let paint(mask-path, px, fills, background: none, tint: none) = {
  let mask = load(mask-path)
  let present = ids(mask)
  let logo = masters.at(master(px))

  let quadrants = fill-rules(fills)
  require(quadrants, logo-ids, "every logo master")

  let backdrop = (("background", (fill: if background == none { "transparent" } else { background.to-hex() })),)
  if "inner-shadow" in present {
    assert(tint != none, message: mask-path + " has an inner shadow, so it needs a tint")
    backdrop.push(("inner-tint", (flood-color: tint.to-hex())))
  }
  require(backdrop, present, mask-path)

  let width = float(mask.attrs.width.trim("px", at: end))
  mask = replace-id(mask, "inner", g => {
    let tx = float(g.attrs.transform.trim("translate(", at: start).split(",").first())
    let size = str(width - 2 * tx)
    let attrs = g.attrs
    let _ = attrs.remove("href", default: none)
    g + (attrs: attrs, children: (logo + (attrs: logo.attrs + (x: "0", y: "0", width: size, height: size)),))
  })

  bytes(xml-to-string(styled(mask, quadrants + backdrop)))
}

// An icon for `platform` at `px` square, as an image sized in points.
#let icon(platform, px, fills, background: none, tint: none) = image(
  paint("/res/masks/" + platform + ".svg", px, fills, background: background, tint: tint),
  format: "svg",
  width: px * 1pt,
  height: px * 1pt,
)

// The bare logo, for surfaces that compose it themselves.
#let logo(fills, variant: "md", ..args) = {
  let quadrants = fill-rules(fills)
  require(quadrants, logo-ids, "every logo master")
  image(bytes(xml-to-string(styled(masters.at(variant), quadrants))), format: "svg", ..args)
}
