#import "palette.typ": *

#let brand-page(body) = {
  // SW→NE. Typst puts 0deg at W→E and winds clockwise (y points down), so the
  // house diagonal is 315deg, not 45deg.
  set page(margin: 0pt, fill: gradient.linear(ink, tw.slate-800, angle: 315deg))
  set text(font: sans, fill: paper)
  body
}

// A rounded slab standing in for a captured screen. `place` positions it from
// an alignment, which its body would otherwise inherit, so the body is pinned
// top-left here. Sized as a fraction of the page, so it follows the surface.
#let device(height: 60%, body) = block(
  width: 84%,
  height: height,
  radius: 3em,
  fill: tw.slate-900,
  stroke: 0.125em + edge,
  clip: true,
  inset: 3em,
  align(top + left, body),
)

// Play's phone canvas is 9:16 and Apple's 6.9-inch is nearer 19.5:9, so a
// screen cannot simply be scaled from one to the other — it has to re-lay out.
// Everything downstream is therefore sized in `em` against this root, which is
// 16pt at Play's 1080pt width and grows with any wider surface.
#let root-size(surface) = surface.width / 67.5

// One claim, set large enough to survive the listing's thumbnail scale, over a
// mocked slice of the app. The band is a fixed fraction of the height so a one-
// and a two-line headline put the slab in the same place.
// `band` and `card` are fractions of the page height, per surface: type scales
// with width while the canvas grows in height too, so the proportions that suit
// 9:16 leave a tall surface slack. They are the knob for that, not the type.
//
// `tracking` is a fraction of the headline's own size, not the root — an `em`
// inside `text(size: ..)` resolves against the size being set there.
#let screen(surface, headline, sub, body) = brand-page({
  set page(width: surface.width, height: surface.height)
  set text(size: root-size(surface))
  block(width: 100%, height: surface.at("band", default: 30%), inset: (x: 5.5em, top: 7em), {
    text(size: 5.5em, weight: 700, tracking: -0.023em, headline)
    v(1.5em)
    text(size: 2.25em, weight: 400, fill: muted, sub)
  })
  align(center, device(height: surface.at("card", default: 60%), body))
})
