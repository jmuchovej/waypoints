---
paths:
  - src-tauri/fastlane/*
---

# Agent Guide — `src-tauri/fastlane/`

Store listing assets. They live under `src-tauri/` rather than `docs/` because
they ship the app, not describe it.

## Layout

| Path                  | Holds                                                  |
| --------------------- | ------------------------------------------------------ |
| `Appfile`, `Fastfile` | the lanes, and the App Store privacy answers           |
| `lib/lib.typ`         | facade; re-exports `lib/` so a locale has one import   |
| `lib/palette.typ`     | colours and font families, all from typewind           |
| `lib/logo.typ`        | the logo painter                                       |
| `lib/parts.typ`       | chips, rows, callouts — every size in `em`             |
| `lib/screen.typ`      | `brand-page`, `device`, `screen`                       |
| `lib/surfaces.typ`    | the store surfaces, and the screens × surfaces fan-out |
| `lib/sheet.typ`       | the `index.html` contact sheet and its CSS             |
| `logo/{md,xs}.svg`    | the bare logo, quadrant ids painted at render time     |
| `<locale>.typ`        | that locale's screens and fixed slots                  |
| `<locale>.toml`       | listing copy, every store in one file                  |
| `metadata/`           | generated — supply's tree, and deliver's `.txt`        |
| `screenshots/`        | generated — deliver's screenshots                      |
| `index.html`          | generated — the review sheet                           |

This _is_ the `fastlane` directory, so supply and deliver resolve their default
paths against it and every path below is written relative to it.

**Nothing renders to a staging directory.** The bundle's output root is this
directory and every `#document(..)` path is a full path under it, so PNGs land
straight where supply and deliver read them — no copy step that can go stale:

| Asset                | Path                                                  |
| -------------------- | ----------------------------------------------------- |
| Play icon            | `metadata/android/<locale>/images/icon.png`           |
| Play feature graphic | `metadata/android/<locale>/images/featureGraphic.png` |
| Play screenshots     | `…/images/phoneScreenshots/<nn>-<slug>.png`           |
| Play release notes   | `metadata/android/<locale>/changelogs/default.txt`    |
| App Store shots      | `screenshots/<locale>/<nn>-<slug>.png`                |
| App Store notes      | `metadata/<locale>/release_notes.txt`                 |

`icon` and `featureGraphic` are the names supply looks for, as are the
screenshot folder names (`sevenInchScreenshots`, `tenInchScreenshots` for
tablets). deliver takes one flat directory per locale and picks the device class
from the image resolution. Both stores order screenshots by filename, hence the
numeric prefix.

The generated trees are gitignored by path, and `*.png`/`*.html`/`*.pdf` by
extension, so a second locale needs no new ignore rule.

## Screens × surfaces

A **screen** is a claim — slug, headline, sub, body — and is store-agnostic. A
**surface** is one store's output slot: directory, pixel size, and the share of
the page its band and card take. `phone-assets` renders every screen onto every
surface, so adding a store is a row in `lib/surfaces.typ`, not a second set of
screens to keep in step.

Play's phone is 9:16 and Apple's 6.9-inch is nearer 19.5:9, so a screen cannot
be scaled from one to the other — it has to re-lay out. Two consequences:

- **Size content in `em`, never `pt`.** `screen` sets a root of 16pt at Play's
  1080pt width and scales it with the surface; anything in `pt` would keep
  Play's type size on a wider canvas.
- **`band` and `card` are per-surface.** Type grows with width while the canvas
  also grows in height, so proportions that suit 9:16 leave a tall surface
  slack. Those two knobs absorb it — not the type scale.

A card clips silently when its content outgrows it. Measure rather than guess:
render the bodies on an auto-height page at the card's interior width and read
the heights off.

```typst
#import "/fastlane/lib/lib.typ": *
#import "/fastlane/en-US.typ": screens
#set page(width: 811.2pt, height: auto, margin: 0pt)  // 84% of 1080, less 2×3em
#for (i, s) in screens.enumerate() { s.body; if i < screens.len() - 1 { pagebreak() } }
```

## Listing copy

One file per locale, not one file per field. `description` is shared by every
store; a store needing its own sets `description` inside its own table.

**The app name is not in there.** It is `productName` from the shipping
flavor's Tauri config, so preview and nightly list under their own names —
read it the way `src/justfile` reads the identifier:

```bash
yq -p toml '.productName' configs/flavors/<flavor>.toml
```

| Key                     | Store     | Limit                       |
| ----------------------- | --------- | --------------------------- |
| `description`           | both      | 4000                        |
| `google-play.short`     | Play      | 80                          |
| `app-store.subtitle`    | App Store | 30                          |
| `app-store.promotional` | App Store | 170                         |
| `app-store.keywords`    | App Store | 100 once joined with commas |

`xfastlane metadata [flavor] [locale]` writes the TOML out into the
layout fastlane reads, and refuses to write anything if a field is over its
limit — naming the field, its length and the cap. The output is generated and
gitignored, so it cannot drift from the TOML:

| Written to                        | Read by   | Default path? |
| --------------------------------- | --------- | ------------- |
| `metadata/android/<locale>/*.txt` | `supply`  | yes           |
| `metadata/<locale>/*.txt`         | `deliver` | yes           |

Those filenames are fixed by fastlane, not chosen here — supply's are
`title`, `short_description`, `full_description` (`AVAILABLE_METADATA_FIELDS`
in `supply/lib/supply.rb`), deliver's are `name`, `subtitle`, `description`,
`keywords`, `promotional_text`.

## Release notes

Every channel gets its own notes, internal testing included — a build whose
notes say nothing teaches its testers nothing. `metadata` renders them with
`git-cliff --latest` through `.config/cliff-store.tera`, a plain-text template
that drops the links, hashes and headings the GitHub release gets, because both
stores strip markup.

| Field                     | Cap  |
| ------------------------- | ---- |
| Play "What's new"         | 500  |
| App Store "What's New"    | 4000 |
| TestFlight "What to Test" | 4000 |

500 is the binding one, so the text is cut on a line boundary and an ellipsis
added rather than severing a bullet. Play reads `changelogs/<versionCode>.txt`
and falls back to `default.txt`, which is what gets written — no need to know
the version code here. The Fastfile reads `release_notes.txt` for TestFlight's
"What to Test".

When a tag carries no changelog-worthy commits the files are **not** written:
their absence is what makes a store keep the previous notes, whereas an empty
file would upload blank ones.

## Rendering

`tooling/bin/xfastlane` owns this, on `PATH` from `.env`:

| Command                              | Does                                      |
| ------------------------------------ | ----------------------------------------- |
| `xfastlane build [locale]`           | renders every store and surface, flattens |
| `xfastlane watch [locale]`           | recompiles on save, no alpha strip        |
| `xfastlane metadata [flavor] [loc]`  | listing copy and release notes            |
| `xfastlane clean`                    | removes everything generated              |
| `xfastlane prepare [flavor] [loc]`   | build + metadata; on Linux, Transporter   |
| `xfastlane publish <platform> [fl.]` | the lane only; refuses without `prepare`  |

A release runs `prepare` once, then `publish` per store. Everything that can
fail without consequence — rendering, copy limits, fetching Transporter — is in
`prepare`, so it fails before the first submission. `publish` never prepares on
its own. It invokes fastlane from the app directory, because fastlane finds its
Fastfile by looking for `./fastlane` in the working directory and
`src-tauri/fastlane` is not the repository root.

On Linux, `prepare` fetches Apple's current Transporter build into
`.transporter/` (an exception under `toolchain/tool-projection`; never pin it),
and `publish ios` points fastlane at it. TestFlight uploads off macOS also need
the `AppStoreInfo.plist` the iOS export writes beside the ipa.

`publish --from <dir>` submits the one aab or ipa under a `release-stage` tree
instead of the build output, handing it to the Fastfile as
`WAYPOINTS_STORE_PACKAGE`. The staged tree holds store objects only, so iOS
takes the plist separately (`--app-store-info`) and `publish` puts the two side
by side in a scratch directory.

Three things the recipe gets right that a bare `typst compile` does not:

- `--format bundle`. `TYPST_FEATURES=bundle` only unlocks `#document(..)`; the
  output format still has to ask for it.
- `--ppi 72`. A `pt` in the source is then a pixel on disk. The default 144
  silently doubles every dimension, and a 2048×1000 feature graphic is rejected.
- Alpha stripping on everything but `app-icon.png`. Both stores reject alpha on
  screenshots and the feature graphic, and both require it on the icon.

`TYPST_ROOT` is the app directory, so an absolute path in a `.typ` reaches
`/fastlane/…` and `/icons/…`.

## One array drives everything

Each `<locale>.typ` ends with a single `assets` array — the fixed slots plus
`phone-assets(screens)` — whose entries carry `group`, `path`, `spec`, an
optional `todo`, and the `body`. A `for` loop emits the documents and
`contact-sheet` builds `index.html` from the same array, so a slot cannot be
rendered without appearing on the sheet.

Open `index-<locale>.html` to review a build: every asset linked and previewed
on a chequerboard, with its store requirement beside it. It sits at the output
root so its `<img src>` paths resolve against the same tree the stores read.

## Brand

- **Gradients run SW→NE.** In Typst that is `angle: 315deg` — 0deg is W→E and
  angles wind clockwise, because y points down. Not 45deg.
- **Colour comes from typewind**, never a literal hex. `lib.typ` binds
  `ink`/`paper`/`muted`/`edge`/`accent`; `index.html` emits the same bindings as
  `oklch()` custom properties, so the page cannot drift from the PNGs.
- **Ask for emoji by name**, via `emoji-font` from `lib.typ` —
  `text(font: emoji-font)[📕]`. Two colour faces are on `TYPST_FONT_PATHS` and
  an unqualified glyph picks one by discovery order. Twemoji is the default
  because it sits on the baseline at the surrounding size; Noto Color Emoji is
  CBDT bitmap and draws oversized and high.
- **The logo is painted at render time.** `logo(variant: "md")` recolours the
  exported SVG's quadrant ids — the same ids `tooling/icongen/flavors/*.toml`
  paints. Do not commit a pre-coloured copy, and do not read icongen's
  `background-inner`/`background-outer` as a radial: it is a flat fill plus an
  inner-shadow tint.

## What a listing may claim

Store copy is a product claim. Check
`.claude/skills/waypoints-architecture/references/architecture.md` §10 before
writing a screen: sync of note content, saved views, formula fields, full-text
search, external importers, and backup/export are **not implemented** and must
not appear.

Both stores also forbid superlatives ("best", "#1"), prices, calls to action
("download now"), award or ranking badges, and time-bound promotions. The UI
shown has to be the UI that ships.

## How many screenshots

Play takes 2–8 phone screenshots and needs 4 at 1080px or more to be eligible
for promotional placement. Four distinct claims beat eight restatements — the
first two or three are all most people scroll past anyway.
