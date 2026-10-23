#import "palette.typ": *

// Everything here is sized in `em` against the root size `screen` derives from
// its surface, so one definition serves canvases of different pixel sizes.

#let chip(label, fill: tw.slate-800, stroke: none) = box(
  radius: 50em,
  fill: fill,
  stroke: stroke,
  inset: (x: 1.125em, y: 0.5625em),
  text(size: 1.5em, weight: 600, fill: paper, label),
)

#let block-row(glyph-fill, title, detail) = grid(
  columns: (1.75em, 1fr),
  column-gutter: 1.625em,
  align: (center + top, left + top),
  // Nudged to the title's optical centre rather than the cell's.
  move(dy: 0.75em, circle(radius: 0.6875em, fill: glyph-fill)),
  {
    text(size: 2em, weight: 600, title)
    h(0.875em)
    text(size: 1.75em, fill: muted, detail)
  },
)

#let check-row(done, label) = grid(
  columns: (2.125em, 1fr),
  column-gutter: 1.5em,
  align: (center + top, left + top),
  move(dy: 0.375em, rect(
    width: 1.75em,
    height: 1.75em,
    radius: 0.4375em,
    fill: if done { accent.at(1) } else { none },
    stroke: if done { none } else { 0.125em + edge },
    align(center + horizon, if done {
      text(size: 1.25em, weight: 700, fill: ink)[✓]
    }),
  )),
  text(size: 1.75em, fill: if done { muted } else { paper }, label),
)

// A Reading on a Landmark: the Feature's name, then its value.
#let field-row(label, value) = grid(
  columns: (15.625em, 1fr),
  column-gutter: 1.5em,
  align: (left + horizon, left + horizon),
  text(size: 1.75em, fill: muted, label),
  value,
)

// A Terrain edge: which way it points, what it reaches, and its kind.
#let edge-row(arrow, title, kind) = grid(
  columns: (2.25em, 1fr, auto),
  column-gutter: 1.25em,
  align: (center + horizon, left + horizon, right + horizon),
  text(size: 1.875em, fill: muted, arrow),
  text(size: 1.875em, weight: 600, title),
  chip(kind, fill: none, stroke: 0.125em + edge),
)

#let callout(accent-fill, body) = block(
  width: 100%,
  radius: 0.75em,
  fill: tw.slate-950,
  stroke: (left: 0.375em + accent-fill),
  inset: (x: 1.75em, y: 1.5em),
  text(size: 1.75em, fill: tw.slate-300, body),
)

#let divider = line(length: 100%, stroke: 0.125em + tw.slate-800)
