// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

#import "palette.typ": *
#import "devices.typ": devices

#let brand-page(body) = {
  // SW→NE. Typst puts 0deg at W→E and winds clockwise (y points down), so the
  // house diagonal is 315deg, not 45deg.
  set page(margin: 0pt, fill: gradient.linear(ink, tw.slate-800, angle: 315deg))
  set text(font: sans, fill: paper)
  body
}

// A rounded slab standing in for a captured screen, for the screens not yet
// ported to a capture. `place` positions it from an alignment, which its body
// would otherwise inherit, so the body is pinned top-left here. Sized as a
// fraction of the page, so it follows the surface.
#let slab(height: 60%, body) = block(
  width: 84%,
  height: height,
  radius: 3em,
  fill: tw.slate-900,
  stroke: 0.125em + edge,
  clip: true,
  inset: 3em,
  align(top + left, body),
)

// Typst cannot ask whether a file exists, so `xfastlane build` passes the PNGs
// that do as `--input files=<json>`, and a missing one is named here rather
// than surfacing as Typst's own file-not-found. Without the input (a bare
// `typst compile`), nothing is checked.
#let present = if "files" in sys.inputs { json(bytes(sys.inputs.files)) } else { none }
#let require(path, hint) = {
  if present != none and path not in present { panic(path + " is absent; " + hint) }
  path
}

// A capture inside its device's frame, `height` tall. The capture is clipped
// to the frame's rounded screen and the frame drawn over it, so the island or
// punch hole occludes the capture the way the hardware does.
#let device(capture, profile, height) = {
  let frame = require(profile.image, "export it into res/ as res/.gitkeep says")
  let capture = require(capture, "mint it with `xfastlane capture`")
  let px = height / profile.size.at(1)
  let screen = profile.screen
  box(width: profile.size.at(0) * px, height: height, {
    place(top + left, dx: screen.x * px, dy: screen.y * px, box(
      width: screen.width * px,
      height: screen.height * px,
      radius: profile.radius * px,
      clip: true,
      image(capture, width: 100%, height: 100%, fit: "cover"),
    ))
    place(top + left, image(frame, height: height))
  })
}

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
//
// `capture` is the path of a capture to frame in the surface's device; a
// screen without one still draws `body` into a slab.
#let screen(surface, headline, sub, body, capture: none) = brand-page({
  set page(width: surface.width, height: surface.height)
  set text(size: root-size(surface))
  block(width: 100%, height: surface.at("band", default: 30%), inset: (x: 5.5em, top: 7em), {
    text(size: 5.5em, weight: 700, tracking: -0.023em, headline)
    v(1.5em)
    text(size: 2.25em, weight: 400, fill: muted, sub)
  })
  let card = surface.at("card", default: 60%)
  if capture == none {
    align(center, slab(height: card, body))
  } else {
    align(center, device(capture, devices.at(surface.device), card * surface.height))
  }
})
