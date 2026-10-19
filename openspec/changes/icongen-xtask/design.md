## Context

See `proposal.md` — Why. What shapes the approach:

The current generator is ~640 lines of TypeScript whose real work is **XML surgery, not rasterization**. It loads a logo master and a platform mask as mutable SVG trees, paints nodes by `id`, rewrites the `feColorMatrix` values inside the mask's inner-shadow filter chain to tint it, splices the logo in as a nested `<svg>` inside the mask's `#inner` node, and only then hands a serialized string to the rasterizer. Every platform writer is a thin loop over a size table around that one pipeline.

Three facts about the tree as it stands:

- `src-tauri/res/` is already populated with the authored inputs (logo masters, masks, palettes, size table, Tauri config fragments), and `src-tauri/assets/` already exists and is empty. `tooling/icongen/` still holds byte-identical duplicates of the inputs, because the generator defaults its source directory to its own location.
- `src-tauri/res/configs/flavors/*.toml` already point at a new, unimplemented output scheme — `res/icons/app/<short-flavor>-<zero-padded>.png` — with flavors shortened (`development` → `dev`). The generator knows neither the flat naming nor the shortening. `src-tauri/icons/` still holds the old `icons/<flavor>/32x32.png` layout.
- `src-tauri/fastlane/lib/logo.typ` already reads `/res/logo/`, so the logo masters are genuinely shared with the marketing renderer. `TYPST_ROOT` is the app directory, so anything under `src-tauri/` is reachable from Typst by absolute path.

The palette duplication is concrete: `fastlane/lib/palette.typ` binds `accent` to four Tailwind colors by hand and `logo.typ` binds the connector fills to another, together restating `res/palettes/stable.toml`'s `[fills.dark]` table. `logo.typ`'s comment points at `tooling/icongen/flavors/*.toml`, a path that no longer exists.

## Goals / Non-Goals

**Goals:**

- Settle the palette contract before any Rust exists, so the token grammar is fixed when the generator is written against it.
- Keep the TypeScript generator working throughout the palette and layout work, so neither depends on the port landing.
- Make the rasterizer substitution a measured decision rather than a discovery.

**Non-Goals:**

- Reorganizing the mask or logo artwork to suit the new renderer. If a mask needs changes to render correctly, that is a finding, not a licence to redraw.
- Abstracting over rasterizers, or keeping the TypeScript generator as a fallback. One generator.
- Generalizing the token grammar beyond what the palettes use.

## Decisions

### 2026-10-19 — Tokens are `-`-joined and resolved directly against the upstream scale

Tokens become `emerald-400` rather than `emerald.400`, and `white/50` keeps its opacity suffix.

The reason is not cosmetic. The Typst marketing renderer already depends on `@preview/typewind:0.2.0`, which exposes Tailwind colors as module members named exactly `emerald-400`. `dictionary(typewind)` converts that module into an indexable dictionary, so a `-`-joined token read from a TOML file is a **direct dictionary key** — no parsing, no per-renderer lookup table, no regex. The Rust side indexes a vendored table under the same spelling.

Verified against the tree on 2026-10-19:

- `dictionary(typewind)` yields 286 colors and is dynamically indexable under Typst 0.15.1.
- `tailwindcss@4.2.2` + chroma-js yields 26 families × 11 shades = 286 entries.
- Spot-checked tokens agree exactly across both, oklch→sRGB conversion included: `slate-200` → `#e2e8f0`, `emerald-400` → `#00d492`, `sky-400` → `#00bcff`, `rose-500` → `#ff2056`, `slate-700` → `#314158`, `slate-600` → `#45556c`. typewind 0.2.0 tracks Tailwind **v4** (v3's `emerald-400` was `#34d399`).
- The end-to-end Typst path works: `toml("/res/palettes/stable.toml")` → resolve → `#00d492 #e2e8f0 #00bcff #e2e8f0 #ffb900 #ff2056`, and `white/50` → `ffffff80`, matching what the TypeScript `resolveToken` produces (`round(0.50 × 255) = 0x80`).

The whole token surface is 19 distinct tokens across four palettes, and the only opacity form in use is `white/50`. The existing regex is far more general than anything the palettes exercise; the new grammar is `<name>` optionally followed by `/<opacity>`, split on `/`.

Two findings that must be carried into implementation:

- **typewind has no `white`, `black`, or `transparent`.** Its 286 entries are families only. `tailwindcss/colors` carries the keyword colors as top-level strings, which is why the current resolver has a string branch for them — and `development.toml`'s entire `fills.mono` table is `white/50`. The Typst resolver needs those keywords added explicitly. This fails at render time, not parse time, if missed.
- **Opacity means "set alpha", and only one Typst primitive says that.** `white/50` means alpha = `0x80` exactly. Measured on 2026-10-20, against an opaque base and against a base already at 50%:

  | base        | `transparentize(50%)` | `opacify(50%)` | `.rgb().components()` → `rgb(r, g, b, 50%)` |
  | ----------- | --------------------- | -------------- | ------------------------------------------- |
  | `#ffffff`   | `#ffffff80`           | `#ffffff`      | `#ffffff80`                                 |
  | `#ffffff80` | `#ffffff40`           | `#ffffffc0`    | `#ffffff80`                                 |

  `opacify` _raises_ opacity, so on an opaque base — which every scale entry is — it is a no-op. Reaching for it would render `development.toml`'s entire `fills.mono` table as opaque white rather than half-transparent, in the one flavor where every fill carries `/50`.

  `transparentize` lands exactly right on an opaque base, but by arithmetic coincidence: it scales by `1 - amount`, which equals the target only because the base alpha is 1. It is correct for today's inputs for a reason that is not the reason the token means.

  So the resolver decomposes and sets alpha explicitly: `.rgb().components()` then `rgb(r, g, b, <opacity>)`. This is idempotent, says what the token says, and works on typewind's oklch-origin colors — `slate-700` decomposes to `(19.35%, 25.53%, 34.35%, 100%)` and recomposes to `#314158` unchanged, or `#31415880` at `/50`. Staying in the color domain also avoids the hex-string surgery an earlier draft of this document proposed.

Alternatives considered:

- _Keep `.` joins and parse on both sides._ Rejected: it reintroduces a parser on each side for no gain, and the `.` spelling collides with TOML's own key syntax in a way that reads confusingly inside a TOML file.
- _Vendor the palette for Typst too._ Rejected: typewind already is the palette, pinned, and provably in agreement. Vendoring twice doubles the drift surface.
- _Have Typst resolve nothing and read pre-resolved hex from a generated file._ Rejected: it makes the palette a generated artifact rather than an authored one, and puts a build step between editing a color and seeing it.

### 2026-10-19 — Only the Rust side vendors the scale, and the vendored copy is generated

Typst reaches the scale through typewind. Rust has no equivalent, so it carries a flat table of 286 `-`-joined names → packed RGB, generated from `tailwindcss/colors` rather than transcribed.

Being generated, it joins the files under the lockstep check and needs a regeneration path. That is the real cost of vendoring — not the ~290 lines.

`tailwindcss` is a caret range (`^4.2.2`) while typewind is pinned at `0.2.0`, so a Tailwind v4 minor that retunes a color desyncs the Rust table from typewind silently. The guard is a test asserting the tokens the palettes actually name still agree — 19 entries today, and it widens on its own as palettes grow, which is what `brand-palette`'s agreement requirement asks for.

Alternative considered: _vendor only the tokens in use._ Rejected — it turns adding a token into a two-step edit, and the cost it saves is trivial.

### 2026-10-19 — `res/` is inputs, `assets/` is outputs, and mobile is the stated exception

`src-tauri/res/` keeps logo masters, masks, palettes, the size table, and the Tauri config fragments. `src-tauri/assets/` takes the generated icons. `src-tauri/icons/` is deleted. The flavor fragments' icon paths move from `res/icons/app/…` to `assets/icons/app/…`.

iOS asset catalogs (`gen/apple/Assets.xcassets/<flavor>.appiconset/`) and Android mipmap resources (`gen/android/app/src/<flavor>/res/mipmap-*/`) cannot move — Xcode and Gradle own those paths. So the rule is "outputs the repository owns live in `assets/`", and `gen/` holds the rest. Writing this down is the point; discovering it mid-port is the failure mode.

Generated icons stay **committed**, unlike the marketing captures. Two reasons: the Tauri bundler needs them present at build time, including in release and Nix builds that have no generation step; and committing them is what makes the lockstep check possible, which is most of the port's value.

### 2026-10-19 — Painting stays XML surgery; the usvg tree is not the place for it

`usvg` resolves `<use>`, filters, and nested `<svg>` into a typed tree that cannot be meaningfully mutated afterward, and there is no API to rewrite a filter primitive's values. So painting happens on raw XML before parsing, exactly as it does now. That calls for a **mutable** XML tree — `roxmltree` is read-only — which keeps the port close to line-for-line against the current `svgson` traversals.

The one subtlety worth stating: the current nested-`<svg>` injection copies the logo's attributes wholesale onto the nested node and then overrides `width`/`height`, so the master's `viewBox` rides along and performs the scaling. Replicating this inexactly yields a correctly-sized, wrongly-cropped logo — a failure that looks like an artwork problem rather than a port bug.

### 2026-10-19 — Output naming and flavor shortening become part of the size record

The flavor fragments already demand a flat `<short-flavor>-<zero-padded>.png` scheme, while the size table still carries the old names (`name = "32x32.png"`). The naming scheme and the long→short flavor map both become data alongside the sizes, rather than string munging in the generator — otherwise the fragments and the generator encode the same convention in two places, which is the problem this change exists to remove.

### 2026-10-19 — The rasterizer decision is gated on measurement, taken first

`resvg` is already a workspace dependency and already in use in `crates/cartograph`. It supports every filter primitive the masks use — `feMorphology`, `feGaussianBlur`, `feOffset`, `feComposite`, `feColorMatrix` — and resolves nested `<svg>` with its own `viewBox`.

Support is not fidelity. The current renderer is librsvg by way of `sharp`; filter-region math and antialiasing differ between implementations. So the first implementation step is a throwaway spike that paints one desktop mask and renders it large, compared against the committed icon. Everything else depends on that answer, and it is cheap to get.

### 2026-10-19 — Encoder choices

- **PNG / lossless WebP**: the `image` crate, already a workspace dependency. Its WebP encoder writes lossless only — which is exactly what the Android layers need (`webp({lossless: true})` today), so the limitation is a match rather than a constraint.
- **ICO**: a dedicated writer crate, replacing `png-to-ico`.
- **ICNS**: a dedicated writer crate. Verified it covers all ten OSTypes the size table names, `ic04` and `ic05` included. One caveat: it types those two as raw-ARGB variants while the current pipeline embeds PNG for every entry. Expect a different construction path for them, or dropping them — macOS falls back to `ic11`.

## Risks / Trade-offs

- **The rasterizer does not reproduce the masks faithfully** → Settled first, by a spike, before any of the port is built. If the filter chains do not survive, the approach changes rather than the artwork.
- **Every committed icon diffs on the first real run** → Accepted and planned for: a single reviewed commit whose diff is understood to be a renderer change, with a visual check at large size, not a byte comparison. This is called out in the proposal's non-goals so it is not mistaken for a regression later.
- **`ic04`/`ic05` cannot be written as PNG** → Contained: they are two of ten entries and macOS degrades to `ic11`. Decide when the ICNS writer is wired, not before.
- **Tailwind drifts from typewind under the caret range** → The agreement test over tokens in use. It cannot catch a drift in an entry no palette names, which is the right scope.
- **The Typst resolver misses the keyword colors** → Known from the probe, and it is a render-time failure in the one flavor whose mono fills are all `white/50`. Covered by resolving every palette during the palette step, not only `stable`.
- **The palette step touches the shipping marketing renderer** → It lands while the TypeScript generator is still live, so a mistake shows up in marketing output with icons unaffected, and vice versa. The two are never in flight together.
- **New crates need the advisory and licence audit** → Four additions, run through the audit the repository already performs on every dependency change.

## Migration Plan

Three stages, each independently revertible. The ordering is the point: the palette contract is settled before anything is written against it, and the rasterizer question is answered before the port is built out.

1. **Palette unification.** Respell tokens `.` → `-` in the four palettes; widen the TypeScript resolver's separator; add the Typst resolver (typewind dictionary plus the keyword colors) and point `palette.typ` and `logo.typ` at the committed palettes, keeping `accent` as a derived view. The TypeScript generator keeps working; marketing output is checked by rendering it. Reverting is a token respelling.

2. **Input/output split.** Move the staged `res/icons/` to `assets/icons/`, repoint the flavor fragments, delete `src-tauri/icons/` and the duplicated inputs under `tooling/icongen/`, and fix the stale config root in `tooling/bin/xtauri`. Still the TypeScript generator, now reading `res/` and writing `assets/`. Reverting is a directory move.

3. **The port.** Spike the rasterizer first and stop if it fails. Then the vendored scale and its generation path, the size/palette records, the painting pipeline, the five platform writers, the manifests, the CLI arm, the `just` recipe, the lockstep check coverage, and finally removal of `tooling/icongen/` and its eight npm dependencies.

Rollback: stages 1 and 2 are ordinary reverts. Stage 3's rollback is restoring the TypeScript generator, which is why it is removed last, in its own commit, after the Rust task has produced a reviewed icon set.

### 2026-10-20 — The menu-bar writer stays inert, and the port preserves that deliberately

The size table carries a `menu-bar` record, the current generator's writer for it is entirely commented out, and four `res/icons/menu/*.png` files sit untracked in the working copy with no code that produces them.

The writer was left inert because the per-platform format requirements are not settled — what a macOS menu-bar extra, a Linux status-notifier item, and a Windows notification-area icon each actually need differs in ways the one `18/36/54` size record papers over, and the record's own comment asserts they are interchangeable without establishing it. That question is what is unresolved, not whether a tray icon is wanted.

So this change ports the _state_, not a guess at the answer: the record stays, the Rust task ships five platform writers rather than six, and nothing reads the record. The untracked PNGs go — they are output with no generator, so they can only drift, and keeping them would make an unanswered question look answered.

Settling the formats and synthesizing the icons is separate work, deliberately scoped out here rather than deferred by omission. Treat the record as a parked question with its sizing rationale attached, not as a slot awaiting a writer.

One wrinkle worth naming, since `app-icons` requires that a manifest cannot reference an unwritten image: the `menu-bar` record is not a manifest and nothing resolves against it, so it does not violate that requirement. A comment in the record is what keeps a reader from mistaking it for a live slot.

Alternatives considered:

- _Delete the record._ Rejected: it discards the sizing rationale and the cross-platform claim worth testing, and the next person re-derives both.
- _Implement the writer now._ Rejected: it would require deciding the per-platform formats as a side effect of a port, which is the decision this change is least equipped to make well.
