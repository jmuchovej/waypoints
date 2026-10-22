#import "@preview/typewind:0.2.0": *

#let fills = (
  light: (
    SW: emerald-400,
    "SW→NW": emerald-50,
    NW: emerald-50,
    "NW→SE": emerald-50,
    SE: emerald-50,
    NE: emerald-50,
  ),
  dark: (
    SW: emerald-400,
    "SW→NW": emerald-50,
    NW: emerald-50,
    "NW→SE": emerald-50,
    SE: emerald-50,
    NE: emerald-50,
  ),
  mono: (
    SW: white,
    "SW→NW": white.transparentize(50%),
    NW: white.transparentize(50%),
    "NW→SE": white.transparentize(50%),
    SE: white.transparentize(50%),
    NE: white.transparentize(50%),
  ),
)

#let app = (
  background-inner: emerald-900,
  background-outer: emerald-800,
)
