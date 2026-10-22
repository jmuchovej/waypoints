#import "@preview/typewind:0.2.0": *

#let fills = (
  light: (
    SW: sky-400,
    "SW→NW": sky-400,
    NW: sky-400,
    "NW→SE": sky-50,
    SE: sky-50,
    NE: sky-50,
  ),
  dark: (
    SW: sky-400,
    "SW→NW": sky-400,
    NW: sky-400,
    "NW→SE": sky-50,
    SE: sky-50,
    NE: sky-50,
  ),
  mono: (
    SW: white,
    "SW→NW": white,
    NW: white,
    "NW→SE": white.transparentize(50%),
    SE: white.transparentize(50%),
    NE: white.transparentize(50%),
  ),
)

#let app = (
  background-inner: sky-900,
  background-outer: sky-800,
)
