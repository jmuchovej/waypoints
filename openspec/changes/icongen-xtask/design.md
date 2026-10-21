## Context

See `proposal.md` — Why. What shapes the approach:

The current generator is ~640 lines of TypeScript whose real work is **SVG surgery, not rasterization**. It loads a logo master and a platform mask as mutable trees, paints nodes by `id`, rewrites the `feColorMatrix` values inside the mask's inner-shadow filter to tint it, splices the logo in as a nested `<svg>` inside the mask's `#inner` node, and hands the serialized result to `sharp` (librsvg). Every platform writer is a thin loop over a size table around that one pipeline.

The tree as it stands:

- `src-tauri/res/` holds the authored inputs: logo masters, masks, TOML palettes, the size table, Tauri config fragments, and a stub `res/icon/masks.typ`. `src-tauri/assets/icons/{app,menu}/` holds the committed icons. `src-tauri/icons/` and `src-tauri/configs/` still exist, and `tooling/icongen/` keeps its own copies of every input, which is what the TypeScript generator actually reads.
- `res/configs/flavors/*.toml` name `res/icons/app/<short-flavor>-<zero-padded>.png`, a path that no longer exists now that the icons sit under `assets/`. `tooling/bin/xtauri` resolves its config root as `$TAURI_APP_PATH/configs`, not `res/configs`.
- The three desktop masks (`macOS`, `linux`, `windows`) are byte-identical apart from `<title>`. `iOS` and `android` are full-bleed squares with no shadows.
- The marketing renderer (`fastlane/lib/`) restates the stable palette by hand in `palette.typ`, and repaints the logo in `logo.typ` with a regex over `id="…" fill="…"`. `TYPST_ROOT` is `src-tauri/`, so `/res/…` resolves from any Typst file.

## Goals / Non-Goals

**Goals:**

- One renderer for every brand surface that draws the logo, so the app icon and the store listing cannot be painted differently.
- No edit of any authored file at render time, and no pattern-matching over artwork.
- A Rust task that knows nothing about colors, sizes, or artwork: it executes a manifest.

**Non-Goals:**

- Redrawing artwork to suit the renderer, beyond the one tint-primitive change.
- Keeping the TypeScript generator as a fallback. One generator.
- Generalizing the renderer beyond the masks and logo masters the repository has.

## Decisions

### 2026-10-21 — Palettes are Typst files, and a palette entry is the color

`res/palettes/<flavor>.typ`, one per flavor, each exporting the same shape:

```typst
#import "@preview/typewind:0.2.0": *

#let fills = (
  light: (SW: emerald-400, "SW→NW": slate-800, NW: sky-400, …),
  dark:  (SW: emerald-400, "SW→NW": slate-200, NW: sky-400, …),
  mono:  (SW: white, "SW→NW": white, …),
)
#let app = (background-inner: slate-700, background-outer: slate-600)
```

A renderer imports a flavor by computed path — `import "/res/palettes/" + flavor + ".typ": fills, app` — which Typst 0.15.1 accepts (verified 2026-10-21).

This retires the token grammar, the resolver, and with them three earlier decisions in this change: `-`-joined tokens, a vendored Tailwind table for Rust, and a drift test between that table and typewind. Nothing outside Typst reads a palette any more, so there is no second consumer to agree with.

Evidence carried forward from the token work, still true and still why typewind is the right color source: typewind 0.2.0 tracks Tailwind **v4** and agrees byte-for-byte with `tailwindcss@4.2.2` + chroma-js across all 286 entries spot-checked (`emerald-400` → `#00d492`, `slate-700` → `#314158`, …). It has **no** `white`/`black`/`transparent`; Typst's built-ins supply those.

Opacity is now written as a Typst expression. Measured 2026-10-20: on an opaque base, `white.transparentize(50%)` yields `#ffffff80` and `opacify(50%)` is a no-op yielding `#ffffff`. `transparentize` is fine to write in a palette because every base is opaque; `opacify` is the trap. resvg's CSS accepts the resulting 8-digit hex (`#ffffff80` rendered at alpha 0.502, verified 2026-10-21).

Alternatives considered:

- _Keep TOML palettes with a token resolver in Typst._ Rejected: it keeps a grammar and a resolver whose only purpose was letting Rust read the same file. With Typst the sole reader, the file may as well hold colors.
- _One `palettes.typ` holding all four flavors._ Rejected: four files keep a flavor's colors reviewable in isolation, and the import-by-path costs nothing.

### 2026-10-21 — Colors reach the artwork through a stylesheet, not by rewriting it

The renderer inserts a `<style>` element as the first child of the mask's root, built from the palette:

```css
[id="NW"] {
  fill: #00bcff;
  stroke: #00bcff;
}
[id="background"] {
  fill: #314158;
}
[id="inner-tint"] {
  flood-color: #45556c;
}
```

CSS outranks presentation attributes, so the files' authored `fill="…"` values become defaults that only an unstyled render shows. `[id="…"]` rather than `#…` because `SW→NW` is not a valid `#` selector without escaping. One stylesheet governs the whole document, the spliced logo included.

The renderer collects every `id` in the mask and logo trees and fails if a stylesheet rule targets one that is absent, which is how `brand-palette`'s "a color that reaches nothing fails" is enforced.

Surface variants are stylesheet variants, not artwork variants. The Android foreground layer is the dark fills with `[id="background"]{fill:transparent}`; the monochrome layers swap in the `mono` fills.

### 2026-10-21 — The logo is spliced as a tree, and serialized with xmlit

Typst's `xml()` loads each SVG as a tree of `(tag, attrs, children)` dictionaries. One generic helper replaces the element with a given `id`; the splice gives `#inner` a single child, the logo's root element with `x`/`y`/`width`/`height` added. The logo keeps its own `viewBox`, which performs the scaling. `#inner`'s placeholder `href` is dropped, as the TypeScript does with Sketch's dummy `xlink:href`.

`image()` accepts a path, string, or bytes — not a tree (verified: "expected path, string, or bytes, found array") — and Typst has no XML encoder, so the tree goes back to text through `@preview/xmlit`'s `xml-to-string`. The only string handling left is reading attribute values: the root `width="512px"` and `#inner`'s `translate(101.5, 101.5)`.

Two properties of `xml()` to know about:

- It drops namespace declarations and their prefixes: `xmlns` vanishes and `xlink:href` arrives as plain `href`. Both are harmless — Typst's SVG loader renders a root with no `xmlns` (byte-identical, verified), and plain `href` is SVG 2. A future mask relying on another namespace would break without an error.
- Logo and mask ids are not disjoint (both define `path-1`, …). The splice puts them in one document, exactly as the TypeScript does today. It renders correctly; it is a hazard to know about when editing either file.

Rendering everything as **one** SVG matters. A layered alternative — mask, then logo placed by Typst, then the inner shadow on top — avoids the splice but rasterizes the logo on its own pixel grid at a fractional offset (101.5 × 256/512 = 50.75px), which measurably softens its edges at small sizes.

Measured 2026-10-21 against the committed icons (share of pixels differing by more than 2%):

| renderer                                   | 256px     | 32px      |
| ------------------------------------------ | --------- | --------- |
| regex surgery / `xml()` + hand serializer  | 1.07%     | 1.57%     |
| Typst-native masks, layered                | 1.25%     | 1.94%     |
| CSS on authored SVGs, layered              | 0.19%     | 0.88%     |
| **CSS on authored SVGs, spliced, one SVG** | **0.01%** | **0.50%** |

Alternatives considered:

- _Regex over the SVG text._ Rejected: brittle against any edit to the artwork, and what the marketing renderer does today.
- _Masks redrawn natively in Typst._ Rejected: Typst has no blur or shadow primitive (`rect` has no `shadow`; `std` has no filter), so the shadows would still be hand-written SVG, and the masks would stop being files a vector editor opens.
- _A hand-written serializer._ Rejected: xmlit does the same job with escaping handled, is in nixpkgs, and is one less thing to maintain.

### 2026-10-21 — The desktop masks tint their inner shadow with `feFlood`

The inner shadow's trailing `feColorMatrix` becomes:

```xml
<feFlood id="inner-tint" flood-color="#475569" result="tint"/>
<feComposite in="tint" in2="shadowInnerInner1" operator="in"/>
```

Two reasons, either sufficient. A matrix's `values` cannot be set by a stylesheet; `flood-color` can. And the matrix was wrong under resvg: filter primitives compute in linearRGB by default, so a matrix carrying sRGB fractions renders the tint too light. The committed icon's rim pixel is `57,72,96`; the matrix version rendered `83,97,119`; the flood version renders `56,72,96`. Most of the "renderer difference" first attributed to librsvg → resvg was this.

The authored `flood-color` is the old matrix's color, so an unstyled render is unchanged. This is the only edit to authored artwork, and it applies to the three desktop masks; `iOS` and `android` have no inner shadow.

### 2026-10-21 — One Typst bundle renders everything into staging; Rust packs, prunes, and places

`res/icon/icons.typ` is a bundle entry point. One `typst compile --format bundle --ppi 72` emits, for every flavor:

- every PNG output, as `#document(path, page(…))` at exact pixel size (`N pt` at 72 ppi is `N px`);
- the member PNGs for each `.icns` and `.ico`, under a staging-only prefix;
- every text output via `#asset`: iOS `Contents.json`, the Android adaptive-icon XML, and the background color resource in ARGB order;
- a `manifest.json` telling Rust what to do: which members pack into which container (with each ICNS member's OSType), which PNGs become lossless WebP, and the path patterns of the owned output set.

Rust then: packs `.icns`/`.ico` from the members; re-encodes the Android layers to lossless WebP with the `image` crate already in the workspace; removes every existing file matching the owned patterns; copies the staged outputs into `src-tauri/`. The staging directory mirrors `src-tauri/`'s layout, so placement is a path-preserving copy.

Bundle export, verified 2026-10-21: it writes nested paths and leaves unrelated files in its root untouched, refuses `..` ("would escape the bundle root"), and **never deletes** — a stale file survives a run. That last property is why pruning is Rust's job, and why staging is used at all: rendering into `target/icongen/` means a failed render cannot leave `src-tauri/` half-written, and the member PNGs have a home that is not a shipped output.

Alternatives considered:

- _Bundle root is `src-tauri/`, writing outputs in place_ (the fastlane pattern). Rejected: member PNGs would need a gitignored corner of the tree, pruning would have to run before a render that might then fail, and a failed run would leave a partial set.
- _Rust reads the size table itself._ Rejected: the size table would then have two readers that must agree on its meaning. With the manifest, Typst is the only reader and Rust executes instructions.

### 2026-10-21 — The xtask shells out to the pinned `typst` CLI

The repository's xtask convention prefers a Rust-native SDK, and Typst is a Rust library. This is a deliberate exception. The marketing pipeline already renders through the `typst` CLI, so both brand renderers run the same binary, with the same flags, fonts and package set that devenv pins. Embedding would mean implementing Typst's `World` (sources, fonts, packages) to reproduce what the CLI already does, around an export format Typst itself marks experimental — and it would let the two renderers drift onto different Typst versions.

Alternative considered: _embed Typst as a library._ Rejected for the reasons above; revisit if bundle export stabilizes as a library API and the marketing pipeline moves with it.

### 2026-10-21 — The marketing renderer paints its logo through the same renderer

The painting functions — stylesheet construction, splice, render — live in `res/icon/lib.typ`, separate from the bundle entry point. `fastlane/lib/logo.typ` imports the bare-logo painter from it and drops its regex; `fastlane/lib/palette.typ` imports `/res/palettes/stable.typ` and derives `accent` from it as a view, rather than restating colors.

### 2026-10-21 — Typst packages are pinned through Nix

typewind and xmlit are fetched from Typst Universe on first use today. `app-icons` requires generation to work offline, and the lockstep check runs in CI. Both are in nixpkgs (`typstPackages.typewind_0_2_0`, `typstPackages.xmlit_0_1_3`), and `typst.withPackages` composes them, so devenv provides a Typst whose package set is fixed. This also makes the marketing build reproducible, which it is not today.

### 2026-10-19 — `res/` is inputs, `assets/` is outputs, and mobile is the stated exception

`src-tauri/res/` keeps logo masters, masks, palettes, the size table, the Typst renderer, and the Tauri config fragments. `src-tauri/assets/icons/app/` takes the generated desktop icons. `src-tauri/icons/` is deleted. The flavor fragments' icon paths become `assets/icons/app/…`.

iOS asset catalogs (`gen/apple/Assets.xcassets/<flavor>.appiconset/`) and Android mipmap resources (`gen/android/app/src/<flavor>/res/…`) cannot move — Xcode and Gradle own those paths. So the rule is "outputs the repository owns outright live in `assets/`". In `gen/`, the owned set is narrow: the whole `<flavor>.appiconset/`, and on Android only the `ic_launcher*` files and `values/ic_launcher_background.xml`, never the rest of the resource tree.

Generated icons stay **committed**: the Tauri bundler needs them at build time, including in release and Nix builds that have no generation step, and committing them is what makes the lockstep check possible.

### 2026-10-19 — Output naming and flavor shortening live in the size record

The flavor fragments demand a flat `<short-flavor>-<zero-padded>.png` scheme while the size table carries the old names (`name = "32x32.png"`). The naming scheme and the long→short flavor map (`development` → `dev`) become data beside the sizes, not string munging in the renderer, so the fragments and the generator do not encode one convention twice. The size table stays TOML, read by Typst's `toml()`.

### 2026-10-21 — Menu-bar icons stay outside the generated set

The size table carries a `menu-bar` record, the current generator's writer for it is commented out, and the four `assets/icons/menu/*.png` are hand-placed and tracked. The writer was left inert because the per-platform format requirements are not settled — what a macOS menu-bar extra, a Linux status-notifier item, and a Windows notification-area icon each need differs in ways the single `18/36/54` record papers over, and the record's comment asserts they are interchangeable without establishing it.

This change ports that state. The record stays, marked as a parked question; no writer is built; and `assets/icons/menu/` is **outside** the owned patterns, so pruning never touches it and the lockstep check does not cover it. That directory holds output-shaped files nothing generates, which is a deliberate, visible exception until the tray change settles the formats and brings them into the generated set.

Alternatives considered:

- _Delete the record._ Rejected: it discards the sizing rationale and the cross-platform claim worth testing.
- _Implement the writer now._ Rejected: it would decide the per-platform formats as a side effect of a port.

### 2026-10-19 — Container encoding

- **ICNS**: a dedicated writer crate. It covers all ten OSTypes the size table names, `ic04` and `ic05` included, but types those two as raw-ARGB variants where the current pipeline embeds PNG for every entry. Expect a different construction path for them, or dropping them — macOS falls back to `ic11`. Decide when the packer is written.
- **ICO**: a dedicated writer crate, replacing `png-to-ico`.
- **Lossless WebP**: the `image` crate, already a workspace dependency; its WebP encoder writes lossless only, which is exactly what the Android layers need.

## Risks / Trade-offs

- **Bundle export is experimental** → The marketing pipeline already depends on it; pinning Typst through Nix means a behavior change arrives as a deliberate version bump, which the lockstep check then exercises.
- **`xml()` drops namespaces** → Harmless for the current artwork. A mask that depended on a non-SVG namespace would degrade without an error; the lockstep check would show the render change.
- **Logo and mask share id names** → Unchanged from today and rendering correctly. The stylesheet targets only ids the palette names, all of which are distinct between logo and mask.
- **Every committed icon diffs on the first real run** → Expected: the tint correction is deliberately visible, and the rasterizer changes. One reviewed commit, checked visually at large and small size.
- **`ic04`/`ic05` cannot be written as PNG** → Two of ten entries; macOS degrades to `ic11`.
- **The xtask depends on a CLI on `PATH`** → The same `typst` the marketing pipeline uses, pinned by devenv; the task fails naming the missing binary rather than falling back.

## Migration Plan

Each stage is independently revertible. The TypeScript generator reads its own copies under `tooling/icongen/`, so it keeps working until the cutover regardless of what changes under `res/`.

1. **Paths.** Point the flavor fragments at `assets/icons/app/`, fix `xtauri`'s config root, delete `src-tauri/icons/` and `src-tauri/configs/`. Reverting is a path change.
2. **Palettes.** Write `res/palettes/*.typ`, pin typewind and xmlit through Nix, point `fastlane/lib/palette.typ` at the stable palette, delete the TOML palettes. Marketing output is checked by rendering it.
3. **Masks.** The `feFlood` edit on the three desktop masks. Unstyled renders are unchanged.
4. **Renderer.** `res/icon/lib.typ` and `res/icon/icons.typ`, emitting the full staged tree and manifest. `fastlane/lib/logo.typ` moves onto the shared painter.
5. **Packer.** The xtask task: render into staging, pack, prune, place.
6. **Cutover.** Generate and review, bring the outputs under the lockstep check, repoint `just icons::*`, then remove `tooling/icongen/` and its npm dependencies in a commit of its own.

Rollback: stages 1–5 are ordinary reverts. Stage 6's rollback is restoring the TypeScript generator, which is why it is removed last, after the new pipeline has produced a reviewed icon set.

## Open Questions

- **The `res/icon/masks.typ` stub.** It was started for the Typst-native mask idea, which this design rejects. Whether it is removed or repurposed (for instance, as the home of per-platform layout data) does not affect the specs or the approach.
