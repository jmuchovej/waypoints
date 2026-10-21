#import "palette.typ": *

// The quadrant ids `tooling/icongen/flavors/*.toml` paints, bound to the same
// palette everything else draws from. Connectors take the light fill: these
// assets sit on dark grounds.
#let logo-fills = (
  NW: accent.at(0),
  SW: accent.at(1),
  SE: accent.at(2),
  NE: accent.at(3),
  "NW→SE": tw.slate-200,
  "SW→NW": tw.slate-200,
)

// Repaints the exported SVG on the way in. icongen does the same job in
// TypeScript, but only ever emits app icons; marketing wants the bare logo.
//
// Hex rather than oklch: resvg has no oklch, and these assets land in sRGB PNGs
// regardless, so nothing survives export that the clip would have lost.
#let logo(variant: "md", fills: logo-fills, ..args) = {
  let painted = read("/fastlane/logo/" + variant + ".svg").replace(
    regex("id=\"([A-Z→]+)\" fill=\"[^\"]*\""),
    m => {
      let id = m.captures.at(0)
      if id in fills { "id=\"" + id + "\" fill=\"" + fills.at(id).to-hex() + "\"" } else { m.text }
    },
  )
  image(bytes(painted), format: "svg", ..args)
}
