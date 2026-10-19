## Why

App icons are the last generated artifact in the repository produced outside the Rust dev-toolchain. A Node script owns them, which costs a native per-platform image dependency in the lockfile and keeps the icons out of the generated-file lockstep check — the one mechanism that would catch an icon whose source moved without it.

Worse, the colors are stated twice. The icon generator resolves brand tokens from a committed palette; the marketing renderer restates the same colors as hand-written bindings. Nothing compares the two, so the icon a user installs and the logo on a store listing can diverge silently, and neither is wrong according to anything.

The directory holding all of this makes no distinction between what a person authored and what a command produced: logo masters, platform masks, palettes, size tables, and the rendered `.icns`/`.ico`/PNG sets all sit in one tree. There is no way to say "regenerate the outputs" because there is no way to name them.

## What Changes

- **One palette source.** A flavor's palette file becomes the only place a brand color is named. Tokens are spelled in a single grammar that indexes the upstream color scale directly, so every renderer resolves a token to the same color without a per-renderer lookup table. The marketing renderer reads the committed palette instead of restating it.

- **Authored inputs separate from generated outputs.** `src-tauri/res/` holds what a person writes: logo masters, platform masks, palettes, size tables, and the Tauri config fragments. `src-tauri/assets/` holds what the generator emits. `src-tauri/icons/` — the current output root — is removed.

- **The generator becomes an xtask task.** Icon generation joins the other dev-toolchain tasks as a Rust task, rasterizing through the SVG renderer already vendored for the app. The `@waypoints/icongen` package and its eight dependencies leave the workspace, the native image dependency among them.

- **Generated icons come under the generated-file lockstep check.** Regenerating in a clean tree produces no diff; a palette or mask edit that lands without its regenerated icons fails the check. This is not possible today.

- **BREAKING**: every flavor's bundle icon paths change, because the output root moves. The flavor config fragments are the only consumers.

### Non-goals

- **Menu-bar and tray icons.** The size table for them is committed but unused and the current generator's entry point for them is inert, because the per-platform format requirements were never settled. This change ports that state rather than guessing at it: the record stays with its sizing rationale, no writer is built, and the untracked PNGs that have no generator go. Settling the formats is its own work.
- **Mobile output locations.** iOS asset catalogs and Android mipmap resources stay where the platform toolchains require them, under the generated mobile projects. Only outputs the repository owns move to `assets/`.
- **The artwork.** Logo masters, platform masks, and the set of flavors are unchanged. This change moves and repaints them; it does not redraw them.
- **The marketing renderer's layout.** Only its color resolution changes. Page composition, listing copy, and capture are untouched.
- **Pixel-identical output.** A different rasterizer will not reproduce the current bytes exactly. The change commits to a reviewed one-time difference, not to preserving the existing renders.

## Capabilities

This change stays inside the `distribution` domain. App icons are part of what a release presents — the same footing as the listing assets a release uploads — and the palette they draw from is shared with every other brand surface, so it is stated once at the domain level rather than inside the icon capability.

A note on the boundary: `distribution`'s charter scopes it to delivery _outside_ the Tauri bundler pipeline, and icons are consumed by that bundler. They are placed here because what is being specified is which assets a release carries and where they come from, not how the bundler consumes them. `toolchain` is the alternative, but its charter forbids constraining app behavior, and which icon a flavor ships is app-visible.

### New Capabilities

- `distribution/app-icons`: what an app icon set is generated from, that every flavor and platform comes from one command over committed sources, that authored inputs and generated outputs are separately addressable, and that the generated set is held in lockstep with its sources.
- `distribution/brand-palette`: that a brand color is named once and resolved identically by every renderer, what a color token is allowed to be, and that a token no renderer can resolve is an error rather than a fallback.

### Modified Capabilities

None. `toolchain/generated-bindings` governs the TypeScript bindings specifically and already speaks of the lockstep check as a repository-wide mechanism it does not own; bringing another artifact under that check changes no requirement of its own.

## Impact

**Code**

- `tooling/icongen/` — removed, package and resources both
- `tooling/xtask/` — a new task, its palette and size-table modules, and its CLI arm
- `src-tauri/res/`, `src-tauri/assets/`, `src-tauri/icons/` — the input/output split
- `src-tauri/res/configs/flavors/*.toml` — bundle icon paths
- `src-tauri/res/palettes/*.toml` — token spelling
- `src-tauri/fastlane/lib/palette.typ`, `lib/logo.typ` — read the committed palette instead of restating it
- `tooling/bin/xtauri` — the config root it resolves is already stale against `res/`
- root `justfile` — the icons module stops pointing at a Node project

**Dependencies**

- Leaves the workspace: the icon generator package and its eight npm dependencies, including the native image library
- Enters the Rust workspace: an ICNS writer, an ICO writer, a mutable XML tree, and a vendored color scale. The SVG rasterizer and the PNG/WebP encoders are already workspace dependencies. New crates need the advisory and license audit the repository already runs.

**Generated artifacts**

- The vendored color scale is itself generated from the upstream package, so it joins the set of files the lockstep check covers and needs a regeneration path.

**Risk**

- The rasterizer swap is the one step that can invalidate the approach: the platform masks drive filter chains whose output is renderer-specific. This is settled by rendering and comparing before the port is built out, not after.
