#import "palette.typ": *
#import "/res/icon/lib.typ" as icon

// Connectors take the light fill: these assets sit on dark grounds.
#let logo-fills = brand.fills.dark

// The bare logo, painted by the same renderer as the app icons.
#let logo(variant: "md", fills: logo-fills, ..args) = icon.logo(fills, variant: variant, ..args)
