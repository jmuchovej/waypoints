#import "@preview/typewind:0.2.0" as tw

#let ink = tw.slate-950
#let paper = tw.slate-50
#let muted = tw.slate-400
// Not `line`: that is Typst's own element function, and shadowing it here
// would take the divider away from every file importing this one.
#let edge = tw.slate-600

#let accent = (tw.sky-400, tw.emerald-400, tw.amber-400, tw.rose-500)

#let sans = "Albert Sans"
#let mono = "Monaspace Argon"
// Named, not left to fallback: two colour emoji faces are on TYPST_FONT_PATHS
// and which one an unqualified glyph picks up is down to discovery order.
#let emoji-font = "Twitter Color Emoji"
