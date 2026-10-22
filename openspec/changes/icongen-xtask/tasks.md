## 1. Paths

The TypeScript generator reads its own copies under `tooling/icongen/` and keeps working throughout. See design.md — `res/` is inputs.

- [x] 1.1 Repoint the `bundle.icon` lists in `src-tauri/res/configs/flavors/*.toml` from `res/icons/app/…` to `assets/icons/app/…`; verify each of the four fragments names five files that exist
- [x] 1.2 Fix `configs-root` in `tooling/bin/xtauri` to resolve `res/configs`, and delete the superseded `src-tauri/configs/`; verify `xtauri dev desktop` and `xtauri build desktop --flavor stable` both resolve their platform and flavor fragments
- [x] 1.3 Delete the old output root `src-tauri/icons/`; verify `rg 'icons/(stable|preview|nightly|development)/'` matches nothing outside build output under `gen/`
- [x] 1.4 Mark the `menu-bar` record in `src-tauri/res/sizes.toml` as a parked question — no generator, per-platform formats unsettled, interchangeability unverified — keeping its sizing rationale; verify its comment cannot be read as describing a live slot

## 2. Palettes

See design.md — palettes are Typst files.

- [x] 2.1 Pin typewind 0.2.0 and xmlit 0.1.3 into devenv's Typst via `typst.withPackages`; verify a probe importing both compiles with the Typst package cache emptied and the network unavailable
- [x] 2.2 Write `src-tauri/res/palettes/{development,nightly,preview,stable}.typ` from the TOML values, each exporting `fills` (`light`/`dark`/`mono`) and `app` (`background-inner`/`background-outer`), using typewind and Typst built-ins with opacity as `transparentize` (never `opacify`); verify a probe importing each by computed path prints hex identical to what the TOML tokens resolve to today, including `#ffffff80` for `white/50`
- [x] 2.3 Point `src-tauri/fastlane/lib/palette.typ` at `/res/palettes/stable.typ`, deriving `accent` as a view; verify no color the palette names is restated in `palette.typ` (the layout's neutral greys are not brand colors and stay), and the marketing bundle renders with unchanged colors
- [x] 2.4 Delete `src-tauri/res/palettes/*.toml`; verify nothing outside `tooling/icongen/` references them

## 3. Masks

See design.md — the desktop masks tint with `feFlood`.

- [x] 3.1 Replace the inner shadow's trailing `feColorMatrix` in `src-tauri/res/masks/{macOS,linux,windows}.svg` with `<feFlood id="inner-tint" flood-color="#475569">` + `<feComposite operator="in">`; verify the three masks stay identical apart from `<title>`, each renders in an independent renderer (librsvg), and resvg and librsvg agree on the unstyled rim pixel (`59,72,92` both — the color-space fix; the match against the committed icon needs the flavor's background and is checked in 4.2)

## 4. Renderer

See design.md — stylesheet, splice, and one Typst bundle.

- [x] 4.1 Add the output naming scheme and the long→short flavor map to `src-tauri/res/sizes.toml`; verify every filename the flavor fragments name is derivable from the record alone
- [x] 4.2 Write `src-tauri/res/icon/lib.typ`: load mask and logo with `xml()`, splice the logo into `#inner` keeping its `viewBox`, insert a stylesheet built from a palette, serialize with xmlit, and render one SVG; pick the `xs` master at or below 64px; verify the stable macOS render at 256px differs from the committed icon in no more than 0.05% of pixels by more than 2%
- [x] 4.3 Make `lib.typ` fail when a stylesheet rule targets an `id` absent from the mask or logo, naming both; verify a palette entry renamed to a missing id fails compilation with that message
- [x] 4.4 Write `src-tauri/res/icon/icons.typ` as the bundle entry point emitting, per flavor, the Linux and desktop PNG outputs and the ICNS/ICO member PNGs at exact pixel size; verify `typst compile --format bundle --ppi 72` writes every file at its declared pixel dimensions
- [x] 4.5 Emit the iOS catalog — one image per distinct pixel size, the tinted 1024px variant from the `mono` fills, and `Contents.json`; verify manifest/image closure in both directions and that no pixel size is rendered twice
- [x] 4.6 Emit the Android layers (foreground with a transparent background, monochrome from the `mono` fills) as PNGs per density, the adaptive-icon XML, and `ic_launcher_background.xml` in ARGB order; verify the background resource matches the flavor's inner background with channels reordered
- [x] 4.7 Emit `manifest.json` declaring container membership (with ICNS OSTypes), the WebP conversions, and the owned output patterns — `assets/icons/app/**`, each `<flavor>.appiconset/**`, and the Android `ic_launcher*` files only; verify `assets/icons/menu/` matches no pattern
- [x] 4.8 Move `src-tauri/fastlane/lib/logo.typ` onto `lib.typ`'s bare-logo painter, dropping its regex; verify the marketing bundle differs from before only at the logo's shape edges — the same-color stroke the app icons have always carried, which the old regex never set — and every asset without the logo is byte-identical

## 5. Packer

- [x] 5.1 Add the ICNS and ICO writer crates and run the advisory and licence audit; verify the new crates bring no advisory, licence, ban, or source finding and no ignore is added (the advisories `cargo deny` reports — RUSTSEC-2023-0071, -2026-0381, -2026-0382 — fail identically on `main`)
- [x] 5.2 Implement the render step by invoking the devenv-pinned `typst` CLI with the same bundle flags the marketing pipeline uses, into `target/icongen/`; verify a failing render leaves `src-tauri/` untouched and a missing `typst` binary fails naming it
- [x] 5.3 Implement packing from the manifest — `.icns` (deciding the `ic04`/`ic05` path and recording it in design.md), `.ico`, and lossless WebP; verify the `.icns` opens in Preview with every declared entry, the `.ico` carries its four sizes, and each WebP decodes to its source PNG's pixels
- [x] 5.4 Implement pruning of the owned patterns followed by a path-preserving copy into `src-tauri/`; verify a slot removed from the size record loses its file, `assets/icons/menu/` and unrelated Android resources are untouched, and nothing outside the owned patterns changes
- [x] 5.5 Wire `cargo xtask icongen` into `tooling/xtask/src/bin/xtask.rs` and `lib.rs` with flavor selection; verify `--help` lists the flavors and an unknown flavor errors naming the valid set

## 6. Cutover

- [x] 6.1 Generate every flavor and commit the result as one reviewed change, understood as a renderer change plus the tint correction; verify each flavor's desktop artifacts, iOS catalog, and Android resources are present and pass a visual check at 1024px and 32px
- [x] 6.2 Generate twice over unchanged sources; verify the second run leaves the working copy clean
- [x] 6.3 Bring the committed icons under the generated-file lockstep check; verify it fails on a palette change without regeneration, on a hand-replaced icon, and on an orphaned output left committed after its slot is removed, and passes when sources and icons are committed together
- [x] 6.4 Repoint the root `justfile`'s `icons` module at the xtask task and drop the Node recipes; verify `just icons::stable` and `just icons::dev` work with no `node_modules` present
- [x] 6.5 Remove `tooling/icongen/` and drop `@waypoints/icongen` and its eight dependencies from the pnpm workspace and catalog, in a commit of its own; verify `pnpm install --frozen-lockfile` succeeds, the lockfile no longer mentions the package or the dependencies only it used (`sharp` remains, transitively, for the website), the Nix frontend's `pnpmDeps` pin is refreshed, and generation still works

## 7. Verification

- [x] 7.1 Build one flavor per desktop platform; verify each bundles its own flavor's icons and that a missing icon fails the build rather than falling back (macOS verified locally — the bundled `.icns` is byte-identical to `assets/`, and a missing icon fails naming its path; Linux and Windows are left to CI's builds)
- [x] 7.2 Change one color in one palette and regenerate both the icons and the marketing bundle; verify both surfaces move and no second edit was needed
- [x] 7.3 Run the full per-commit hook set; verify clippy, cargo-test, and treefmt are green and `just check-generated` passes
