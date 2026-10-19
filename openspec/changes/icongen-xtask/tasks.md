## 1. Palette unification

Lands while the TypeScript generator is still live, so a mistake here shows up in marketing output with icons unaffected. See design.md — Decisions, tokens are `-`-joined.

- [ ] 1.1 Respell every token in `src-tauri/res/palettes/*.toml` from `family.shade` to `family-shade`, leaving `white`, `black`, and the `/<opacity>` suffix untouched; verify all four files parse and that `grep -c '\.[0-9]'` over them returns zero matches
- [ ] 1.2 Widen the TypeScript resolver's token grammar in `tooling/icongen/icongen.ts` to `<name>[/<opacity>]` split on `/`, indexing the flattened Tailwind scale by the `-`-joined name; verify `just icons::stable` and `just icons::dev` both succeed and produce icons byte-identical to the current committed set
- [ ] 1.3 Add a Typst token resolver to `src-tauri/fastlane/lib/palette.typ` built on `dictionary(typewind)` plus explicit `white`/`black` bindings, applying opacity by decomposing with `.rgb().components()` and recomposing via `rgb(r, g, b, <opacity>)` — never `opacify` or `transparentize`, per design.md; verify it resolves every token in all four palettes (not just `stable`) by compiling a probe that touches each `fills.*` table and `app.*` key, and that `white/50` yields `#ffffff80` rather than `#ffffff`
- [ ] 1.4 Point `palette.typ`'s `accent` and `logo.typ`'s `logo-fills` at the committed palette through that resolver, keeping `accent` as a derived view rather than dropping it, and fix `logo.typ`'s stale comment pointing at `tooling/icongen/flavors/*.toml`; verify no hand-written color value remains in either file and that `grep -n 'tw\.' src-tauri/fastlane/lib/*.typ` shows no direct color member access outside the resolver
- [ ] 1.5 Render the marketing bundle for one store and compare the logo's painted quadrants against the committed `stable` icon at large size; verify the quadrant colors match `#00d492`/`#e2e8f0`/`#00bcff`/`#ffb900`/`#ff2056` as resolved in design.md

## 2. Input/output split

Still the TypeScript generator, now reading `res/` and writing `assets/`. See design.md — Decisions, `res/` is inputs.

- [ ] 2.1 Move the staged `src-tauri/res/icons/` to `src-tauri/assets/icons/` and delete the untracked `src-tauri/res/icons/menu/*.png`; verify `src-tauri/res/` contains no generated artifact and `src-tauri/assets/icons/app/` holds all twenty committed icons
- [ ] 2.2 Repoint the `bundle.icon` lists in `src-tauri/res/configs/flavors/*.toml` from `res/icons/app/…` to `assets/icons/app/…`; verify each of the four fragments names five existing files by running a path-existence check over every entry
- [ ] 2.3 Point the TypeScript generator's default source and output directories at `src-tauri/res/` and `src-tauri/assets/icons/app/`, and teach it the flat `<short-flavor>-<zero-padded>` output naming and the long→short flavor map as data beside the size record rather than as string munging; verify a run for each flavor writes exactly the filenames its config fragment names, with no leftovers
- [ ] 2.4 Delete the duplicated inputs under `tooling/icongen/` (`icon/`, `masks/`, `flavors/`, `sizes.toml`) now that nothing defaults to them; verify `just icons::stable` still succeeds reading only from `src-tauri/res/`
- [ ] 2.5 Delete the old output root `src-tauri/icons/`; verify no file in the tree references it (`rg 'icons/(stable|preview|nightly|development)/'` returns only build-output paths under `gen/`)
- [ ] 2.6 Fix the stale config root in `tooling/bin/xtauri` so `configs-root` resolves to `res/configs`, and remove the superseded `src-tauri/configs/`; verify `xtauri dev desktop` and `xtauri build desktop --flavor stable` both resolve their platform and flavor fragments
- [ ] 2.7 Mark the `menu-bar` record in `src-tauri/res/sizes.toml` as a parked question — no generator, per-platform formats unsettled — keeping its existing sizing rationale and flagging the interchangeability claim as unverified; verify the record is unreferenced by any writer and that its comment cannot be read as describing a live slot

## 3. Rasterizer spike (gate)

Stop here and reassess if this fails. See design.md — Decisions, the rasterizer decision is gated on measurement.

- [ ] 3.1 Write a throwaway Rust binary that paints `src-tauri/res/masks/macOS.svg` with the `stable` dark fills and the inner/outer backgrounds by raw-XML surgery, splices in `res/logo/md.svg` as a nested `<svg>`, and renders at 1024px through `resvg`; verify it produces a PNG without a usvg parse or filter error
- [ ] 3.2 Compare that render against the 1024px entry extracted from the committed `assets/icons/app/stable.icns`; verify the squircle silhouette, the inner-shadow tint, and the logo's position and crop match on visual inspection, and record the finding in design.md
- [ ] 3.3 Repeat 3.1 at 32px with `res/logo/xs.svg` to exercise the small-size master and the `feMorphology` path; verify the connectorless master is selected and the output is legible

## 4. Vendored color scale

- [ ] 4.1 Add a generator that emits a flat Rust table of all 286 `-`-joined Tailwind names to packed RGB from `tailwindcss/colors`, as a generated file with a header marking it as such; verify it emits 286 entries and that `slate-200`/`emerald-400`/`sky-400`/`rose-500` match the values recorded in design.md
- [ ] 4.2 Add the keyword colors (`white`, `black`, `transparent`) to the Rust resolver outside the vendored table, mirroring the Typst side; verify resolving `white/50` yields alpha `0x80`
- [ ] 4.3 Implement the Rust token resolver over that table with the `<name>[/<opacity>]` grammar, erroring and naming the token on an unresolvable name with no fallback; verify unit tests cover a bare name, an opacity-qualified name, a keyword color, and an unknown token
- [ ] 4.4 Add the drift guard asserting every token any palette in `src-tauri/res/palettes/` names resolves to the same value in the vendored table as upstream, widening automatically as palettes grow; verify the test fails when an entry in the vendored table is perturbed
- [ ] 4.5 Bring the vendored table under the generated-file lockstep check; verify `just check-generated` fails on a hand-edit to it and passes after regeneration

## 5. Rust generation pipeline

- [ ] 5.1 Add the new workspace dependencies (ICNS writer, ICO writer, mutable XML tree) and run the advisory and licence audit; verify `cargo deny check` passes and no new ignore is added without a recorded reason
- [ ] 5.2 Implement `sizes.rs` deserializing `res/sizes.toml` into typed per-platform records, including the iOS idiom/scale/size triples and the Android densities; verify a test round-trips the committed file and rejects a record missing a required field
- [ ] 5.3 Implement `palette.rs` deserializing `res/palettes/<flavor>.toml` into the three themed fill sets plus the inner/outer backgrounds, resolved through the task-4 resolver; verify a test loads all four flavors and asserts `stable`'s dark fills against the values in design.md
- [ ] 5.4 Implement the XML painting pipeline — paint logo and mask nodes by `id`, paint `#background`, retint the `#inner-shadow` filter's trailing `feColorMatrix`, and splice the logo into `#inner` preserving the master's `viewBox` so scaling is unchanged; verify unit tests assert each mutation against a fixture, including that the nested `<svg>` carries the master's `viewBox` and an overridden `width`/`height`
- [ ] 5.5 Implement the source-master selection rule (connectorless master at or below 64px, full artwork above) and the shared render entry point returning a pixmap; verify a test asserts the crossover at 64 and 65
- [ ] 5.6 Implement the Linux and Windows writers (PNG set, multi-resolution ICO); verify both produce the filenames the `stable` config fragment names and that the ICO carries the four declared sizes
- [ ] 5.7 Implement the macOS writer, deciding the `ic04`/`ic05` construction path (raw-ARGB variants or dropped with fallback to `ic11`) and recording the choice in design.md; verify the emitted `.icns` opens in Preview and carries every entry the writer claims
- [ ] 5.8 Implement the iOS writer, emitting one image per distinct pixel size, the tinted 1024px variant, and a `Contents.json` whose every entry names an image written in the same run; verify a test asserts manifest/image closure in both directions and that no pixel size is rendered twice
- [ ] 5.9 Implement the Android writer, emitting foreground and monochrome lossless WebP layers per density plus the adaptive-icon XML and the background color resource in ARGB order; verify the emitted background color matches the flavor's inner background with channels reordered
- [ ] 5.10 Wire the task's CLI arm into `tooling/xtask/src/bin/xtask.rs` and its module into `lib.rs`, with flavor, theme, and platform selection; verify `cargo xtask icongen --help` lists them and an unknown platform errors naming the valid set

## 6. Cutover

- [ ] 6.1 Generate every flavor with the Rust task and commit the result as one reviewed change, with the diff understood as a renderer substitution; verify each flavor's five desktop artifacts plus its iOS catalog and Android mipmaps are present and that a visual check at 1024px and 32px passes for all four flavors
- [ ] 6.2 Confirm determinism by generating twice over unchanged sources; verify the second run leaves the working copy clean
- [ ] 6.3 Bring the committed icons under the generated-file lockstep check; verify the check fails when a palette token is changed without regeneration, fails when an icon is hand-replaced, and passes when source and icons are committed together
- [ ] 6.4 Repoint the root `justfile`'s `icons` module at an xtask-dispatching recipe and drop the Node-project recipes; verify `just icons::stable`, `just icons::dev`, and `just icons::clean` work with no `node_modules` present
- [ ] 6.5 Remove `tooling/icongen/` entirely and drop `@waypoints/icongen` and its eight dependencies from the pnpm workspace and catalog, in a commit of its own; verify `pnpm install` succeeds, the lockfile no longer mentions the native image library, and icon generation still works from a clean checkout

## 7. Verification

- [ ] 7.1 Build one flavor per desktop platform and confirm each bundles its own icons; verify the bundled icon matches the flavor built and that naming a missing icon fails the build rather than falling back
- [ ] 7.2 Run the full per-commit hook set over the change; verify clippy, cargo-test, and treefmt are green and `just check-generated` passes
- [ ] 7.3 Render the marketing bundle and generate the icons from the same palette edit; verify a single token change moves both surfaces and that no second edit was needed anywhere
