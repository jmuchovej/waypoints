## Why

App icons are the last generated artifact in the repository produced outside the dev-toolchain. A Node script owns them, which costs a native per-platform image dependency in the lockfile and keeps the icons out of the generated-file lockstep check — the one mechanism that would catch an icon whose source moved without it.

Worse, the colors are stated twice. The icon generator resolves brand colors from a committed palette; the marketing renderer restates the same colors as hand-written bindings and repaints the logo with its own pattern-matching. Nothing compares the two, so the icon a user installs and the logo on a store listing can diverge silently, and neither is wrong according to anything.

The directory holding all of this makes no distinction between what a person authored and what a command produced: logo masters, platform masks, palettes, size tables, and the rendered `.icns`/`.ico`/PNG sets all sit in one tree. There is no way to say "regenerate the outputs" because there is no way to name them.

## What Changes

- **Palettes become Typst.** Each flavor's palette is a Typst file declaring its colors as Typst values, all four with the same shape. There is no token grammar and no resolver: a palette entry _is_ the color. Every brand surface — app icons and marketing alike — imports the palette rather than restating it.

- **Icons are rendered by Typst, from the authored artwork, without editing it.** The masks and logo masters stay authored SVGs. A renderer loads them as trees, places the logo inside the mask's `#inner` node, and colors everything by inserting a stylesheet built from the palette. No attribute is rewritten and no pattern-matching touches the artwork. The marketing renderer paints its logo through the same renderer.

- **Rust packages and places.** A new xtask task runs the Typst render into a staging directory, packs the container formats Typst cannot write (`.icns`, `.ico`, lossless WebP), removes outputs the generator no longer produces, and places the result. It holds no knowledge of sizes, colors, or artwork — Typst hands it a manifest of what to pack and where everything goes.

- **Authored inputs separate from generated outputs.** `src-tauri/res/` holds what a person writes: logo masters, platform masks, palettes, the size table, the renderer, and the Tauri config fragments. `src-tauri/assets/` holds what the generator emits. `src-tauri/icons/` — the old output root — is removed.

- **Generated icons come under the generated-file lockstep check**, including the case of an output that should no longer exist. This is not possible today.

- **One authoring change to the desktop masks.** Their inner-shadow tint moves from a color matrix to a flood color, the one form a stylesheet can set. It also corrects a color-space error that made the tint render too light.

- **BREAKING**: every flavor's bundle icon paths change, because the output root moves. The flavor config fragments are the only consumers.

### Non-goals

- **Menu-bar and tray icons.** The size table for them is committed but unused, because the per-platform format requirements were never settled, and the PNGs under `assets/icons/menu/` are hand-placed. This change ports that state rather than guessing at it: no writer is built, and the hand-placed files sit outside the generated set. Settling the formats is its own work.
- **Mobile output locations.** iOS asset catalogs and Android mipmap resources stay where the platform toolchains require them, under the generated mobile projects. Only outputs the repository owns outright move to `assets/`.
- **The artwork.** Logo masters, mask geometry, and the set of flavors are unchanged. Beyond the tint primitive, nothing is redrawn.
- **The marketing renderer's layout.** Only where its colors and its painted logo come from changes. Page composition, listing copy, and capture are untouched.
- **Pixel-identical output.** The rasterizer changes, and the tint correction is visible. The change commits to a reviewed one-time difference, not to preserving the existing renders.

## Capabilities

This change stays inside the `distribution` domain. App icons are part of what a release presents — the same footing as the listing assets a release uploads — and the palette they draw from is shared with every other brand surface, so it is stated once at the domain level rather than inside the icon capability.

A note on the boundary: `distribution`'s charter scopes it to delivery _outside_ the Tauri bundler pipeline, and icons are consumed by that bundler. They are placed here because what is being specified is which assets a release carries and where they come from, not how the bundler consumes them. `toolchain` is the alternative, but its charter forbids constraining app behavior, and which icon a flavor ships is app-visible.

### New Capabilities

- `distribution/app-icons`: what an app icon set is generated from, that every flavor and platform comes from one command over committed sources, that authored inputs and generated outputs are separately addressable, and that the generated set — including what it no longer contains — is held in lockstep with its sources.
- `distribution/brand-palette`: that a brand color is named once and every brand surface takes it from there, that every flavor's palette has the same shape, and that a palette color which reaches nothing is an error rather than a silent no-op.

### Modified Capabilities

None. `toolchain/generated-bindings` governs the TypeScript bindings specifically and already speaks of the lockstep check as a repository-wide mechanism it does not own; bringing another artifact under that check changes no requirement of its own.

## Impact

**Code**

- `tooling/icongen/` — removed, package and resources both
- `tooling/xtask/` — a new task and its CLI arm
- `src-tauri/res/icon/` — the Typst renderer and the bundle entry point
- `src-tauri/res/palettes/` — `*.toml` replaced by `*.typ`
- `src-tauri/res/masks/{macOS,linux,windows}.svg` — the inner-shadow tint primitive
- `src-tauri/res/`, `src-tauri/assets/`, `src-tauri/icons/` — the input/output split
- `src-tauri/res/configs/flavors/*.toml` — bundle icon paths
- `src-tauri/fastlane/lib/palette.typ`, `lib/logo.typ` — import the palette and the shared painter instead of restating them
- `tooling/bin/xtauri` — the config root it resolves is stale against `res/`
- root `justfile` — the icons module stops pointing at a Node project

**Dependencies**

- Leaves the workspace: the icon generator package and its eight npm dependencies, including the native image library
- Enters the Rust workspace: an ICNS writer and an ICO writer. PNG decoding and lossless WebP encoding use a crate already in the workspace. New crates need the advisory and licence audit the repository already runs.
- Typst packages: the color scale the marketing renderer already uses, plus an XML serializer. Both are pinned through Nix so rendering needs no network.

**Risk**

- Bundle export is marked experimental by Typst. The marketing pipeline already depends on it, so this widens an accepted dependency rather than introducing one.
