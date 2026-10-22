#import "@preview/typewind:0.2.0": *

#let fills = (
  light: (
    SW: amber-200,
    "SW→NW": amber-200,
    NW: amber-200,
    "NW→SE": amber-200,
    SE: amber-200,
    NE: amber-50,
  ),
  dark: (
    SW: amber-200,
    "SW→NW": amber-200,
    NW: amber-200,
    "NW→SE": amber-200,
    SE: amber-200,
    NE: amber-50,
  ),
  mono: (
    SW: white,
    "SW→NW": white,
    NW: white,
    "NW→SE": white,
    SE: white,
    NE: white.transparentize(50%),
  ),
)

#let app = (
  background-inner: amber-500,
  background-outer: amber-600,
)
