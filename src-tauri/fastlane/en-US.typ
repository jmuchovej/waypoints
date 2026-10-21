#import "/fastlane/lib/lib.typ": *

// Typst cannot see its own filename, so the locale is declared rather than
// derived. Every output path is built from it.
#let locale = "en-US"

#let listing = [Mobile Assets, #raw(locale)]
#set document(title: listing)

// The same file the stores read their copy from, so a line that appears both
// in the listing and on an asset is written once.
#let copy = toml("/fastlane/" + locale + ".toml")

// Store-agnostic. Each is rendered onto every phone surface in `lib/surfaces`,
// so adding a store adds no screens here. Sizes are `em` against the root
// `screen` derives from the surface — never `pt`, or a wider canvas would keep
// Play's type size.
#let screens = (
  (
    slug: "blocks",
    headline: [Every note is a block],
    sub: [Headings, lists, callouts and code — rearranged by dragging, never by reformatting.],
    body: {
      text(size: 2.625em, weight: 700)[Field notes]
      v(1.625em)
      text(size: 1.75em, fill: muted)[
        Survey of the north ridge. Each paragraph below is its own
        block, addressable on its own and movable without touching
        the text around it.
      ]
      v(2.375em)
      block-row(accent.at(0), "Heading", "structures the page")
      v(1.625em)
      block-row(accent.at(1), "Checklist", "tracks what is left")
      v(1.625em)
      block-row(accent.at(2), "Callout", "sets a passage apart")
      v(1.625em)
      block-row(accent.at(3), "Code", "keeps whitespace intact")
      v(2.375em)
      callout(accent.at(2))[
        A block carries its own identity, so a link to it survives
        every edit made to the page around it.
      ]
      v(1.875em)
      block(
        width: 100%,
        radius: 0.75em,
        fill: tw.slate-950,
        inset: (x: 1.75em, y: 1.5em),
        text(font: mono, size: 1.625em, fill: muted, "waypoints://mark/0f3a…"),
      )
      v(2.75em)
      divider
      v(2.25em)
      text(size: 2em, weight: 600)[Before the next ascent]
      v(1.625em)
      check-row(true, "Log the ridge timings")
      v(1.25em)
      check-row(true, "Photograph the cairn")
      v(1.25em)
      check-row(false, "Redraw the approach sketch")
    },
  ),
  (
    slug: "local-first",
    headline: [Your device, \ and nowhere else],
    sub: [No cloud account, no API key, no telemetry. The database is a file you own.],
    body: {
      v(1fr)
      align(center, {
        logo(width: 13.75em)
        v(3em)
        text(size: 2.125em, weight: 600)[Stored on this device]
        v(1em)
        text(size: 1.625em, fill: muted)[No account to create, nothing to sign in to]
      })
      v(1fr)
    },
  ),
  (
    slug: "charts",
    headline: [Pages that know \ their own shape],
    sub: [A Chart says which fields a page carries. Fill them in and they stay typed — dates sort, numbers add up.],
    body: {
      chip[#text(font: emoji-font)[📕] Book]
      v(1.375em)
      text(size: 2.875em, weight: 700)[The Living Mountain]
      v(2.625em)
      field-row("Author", text(size: 1.75em)[Nan Shepherd])
      v(1.875em)
      field-row("Status", chip("Reading", fill: accent.at(0).darken(30%)))
      v(1.875em)
      field-row("Started", text(size: 1.75em)[12 March 2026])
      v(1.875em)
      field-row("Pages", text(size: 1.75em)[108])
      v(1.875em)
      field-row("Related", chip("Cairngorms", fill: none, stroke: 0.125em + edge))
      v(2.875em)
      divider
      v(2.375em)
      text(size: 2em, weight: 600)[Notes]
      v(1.5em)
      text(size: 1.75em, fill: muted)[
        Below the fields the page is still a page — the same blocks, in
        the same editor. Fields describe the thing; the body is where
        you think about it.
      ]
      v(2em)
      callout(accent.at(1))[
        Charts ship for pages, people, meetings, journals and books —
        or write your own.
      ]
    },
  ),
  (
    slug: "terrain",
    headline: [Links are edges, \ not text],
    sub: [Every connection has a direction and a kind, so a page can show what points back at it.],
    body: {
      text(size: 2.875em, weight: 700)[The Living Mountain]
      v(2.75em)
      text(size: 1.625em, weight: 600, fill: muted)[LINKS OUT]
      v(1.75em)
      edge-row("→", "Cairngorms", "Reference")
      v(1.5em)
      edge-row("→", "Cairngorms", "Custom")
      v(1.5em)
      edge-row("→", "Reading notes", "Child")
      v(3.375em)
      text(size: 1.625em, weight: 600, fill: muted)[LINKED FROM]
      v(1.75em)
      edge-row("←", "Winter reading", "Reference")
      v(1.5em)
      edge-row("←", "Books, 2026", "Child")
      v(1.5em)
      edge-row("←", "Nan Shepherd", "Reference")
      v(3.375em)
      callout(accent.at(0))[
        Cairngorms appears twice above, under two different kinds.
        Each edge is its own thing, so neither overwrites the other
        and both can be followed back.
      ]
    },
  ),
)

// Fixed canvases with no counterpart on the other store: Play alone takes a
// feature graphic, and Apple's marketing icon comes from icongen's appiconset
// rather than from here.
//
// `icon` and `featureGraphic` are the names supply looks for; anything else in
// this directory is ignored.
#let play-images = "metadata/android/" + locale + "/images"

#let play-only = (
  (
    group: "Google Play — icon & feature graphic",
    path: play-images + "/icon.png",
    spec: "512×512 · 32-bit PNG, full bleed, no corner radius",
    // Play rounds to a 30% radius and adds its own shadow, so the artwork is
    // square and effect-free; the logo stays inside the ~16% safe zone that
    // rounding eats into.
    body: {
      set page(width: 512pt, height: 512pt, margin: 0pt, fill: tw.slate-700)
      place(center + horizon, logo(width: 68%))
    },
  ),
  (
    group: "Google Play — icon & feature graphic",
    path: play-images + "/featureGraphic.png",
    spec: "1024×500 · JPEG or 24-bit PNG, no alpha",
    // Play re-crops this to other ratios from the centre, so the lockup is
    // centred with bleed all round. A promo video would overlay a play button
    // here; the lockup has to shift off-centre on the day one is added.
    body: brand-page({
      set page(width: 1024pt, height: 500pt)
      place(
        center + horizon,
        grid(
          columns: (156pt, auto),
          column-gutter: 40pt,
          align: (center + horizon, left + horizon),
          logo(width: 156pt),
          {
            text(size: 82pt, weight: 700, tracking: -2.5pt)[Waypoints]
            linebreak()
            v(6pt)
            text(size: 32pt, fill: muted, copy.at("app-store").subtitle)
          },
        ),
      )
    }),
  ),
)

#let assets = play-only + phone-assets(screens, locale)

#for a in assets {
  document(a.path, a.body)
}

// Sits at the output root so the `<img src>` paths above resolve from it.
#document("index.html", title: listing, contact-sheet(assets))
