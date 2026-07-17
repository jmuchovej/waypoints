// The Waypoints Emergency Kit — a printable recovery document in the
// spirit of 1Password's: the recovery phrase (words + QR), the member ID,
// and how to recover, meant to be printed or stored somewhere safer than
// the device itself.
//
// DESIGN/PREVIEW: this file compiles standalone (`typst watch
// emergency-kit.typ`, tinymist, …) using the inline demo data below and
// the committed `demo-qr.svg` beside it.
//
// RUNTIME: index.ts compiles this same source with the real values passed
// through `sys.inputs.kit` (a JSON object) and the real QR shadow-mapped
// over the `demo-qr.svg` path — the demo assets never end up in a real kit.
#import "@preview/typewind:0.2.0" as tw

#let kit = if "kit" in sys.inputs {
  json(bytes(sys.inputs.kit))
} else {
  (
    memberId: "d3adbeefd3adbeefd3adbeefd3adbeefd3adbeefd3adbeefd3adbeefd3adbeef",
    words: (
      "abandon",
      "ability",
      "able",
      "about",
      "above",
      "absent",
      "absorb",
      "abstract",
      "absurd",
      "abuse",
      "access",
      "accident",
      "account",
      "accuse",
      "achieve",
      "acid",
      "acoustic",
      "acquire",
      "across",
      "act",
      "action",
      "actor",
      "actress",
      "actual",
    ),
    created: "01 Jan 2026",
  )
}

#set page(paper: "us-letter", margin: 56pt)
#set text(font: "Brandon Text", size: 10.5pt, fill: rgb("#3c3c3c"))
#show raw: set text(font: "Monaspace Krypton", size: 0.875em)

#let kitbox = block.with(
  width: 100%,
  stroke: 0.1em + tw.slate-300,
  fill: tw.slate-50,
  radius: 0.625em,
  inset: 1.500em,
  below: 1.750em,
)

#block(width: 100%, radius: 1em, fill: tw.slate-800, grid(
  columns: (1fr, auto),
  align: horizon,
  block(height: 5em, image("../../../public/logos/wordmark-dark-md.svg")),
  grid.cell(
    inset: (right: 1em, rest: 0em),
    smallcaps(strong(delta: 200, text(fill: tw.slate-100, size: 1em)[EMERGENCY KIT])),
  ),
))
#block(width: 100%, align(horizon + center, text(fill: tw.slate-600)[Created on #kit.created]))

If you're ever logged out of your account, you'll need this recovery phrase to log back in. *We cannot access this, so if you lose all your devices _and this document_, your data is unrecoverable.*

#block(width: 100%, fill: tw.rose-50, stroke: 0.1em + tw.rose-200, radius: 1em, inset: 1.500em)[
  #set text(fill: tw.rose-500)

  #text(weight: "black")[
    Anyone who has this phrase #underline(offset: 0.125em)[is you].
  ] _Never share it and never enter it anywhere unless you're setting up a device!_
]

+ Get your Emergency Kit off your computer and print out a hardcopy.
+ Store it somewhere safe (e.g., with your birth certificate, passport, a password manager – among many other safe places).

#kitbox({
  let cell-h = 1.500em
  let n-cols = 4
  let n-rows = int(kit.words.len() / n-cols)
  let qr-width = 6 * cell-h
  grid(
    columns: (1fr,) * n-cols + (auto,),
    rows: (cell-h,) * n-rows,
    column-gutter: 1.000em,
    row-gutter: 0.625em,
    align: horizon,
    // stroke: 0.1em + tw.red-500,
    grid.cell(colspan: n-cols)[*Recovery Phrase*],
    grid.cell(x: n-cols)[*Setup Code*],
    grid.cell(x: n-cols, box(width: cell-h + qr-width, [
      #set text(size: 0.625em)
      Scan this code from the Waypoints app to get setup quickly and easily!
    ])),
    grid.cell(rowspan: n-rows - 1, x: n-cols, align: horizon + start, image("demo-qr.svg", width: qr-width)),
    ..kit.words.map(raw),
  )
})

#kitbox[
  *Member ID*
  #h(0.75em)
  #text(size: 0.75em, fill: tw.slate-500)[
    Your public identity — purely a sanity check. After recovering, Settings → Sync should
    show exactly this ID; if it differs, you recovered a different identity (wrong kit or
    wrong phrase).
  ]
  #v(0.25em)
  #raw(kit.memberId)
]

#text(size: 11pt, weight: "bold", fill: rgb("#141414"))[How to recover]
#v(2pt)
+ Install Waypoints on your new device.
+ During setup, choose "Recover from phrase".
+ Enter the recovery phrase above (or scan the QR code and paste it).
+ Reconnect to your Station in Settings → Sync to pull your data back down.

#v(1fr)
#text(size: 8.5pt, fill: rgb("#969696"))[Waypoints never stores this phrase and cannot recover it for you.]
