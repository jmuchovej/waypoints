## Why

Waypoints is only installable through the Tauri bundler pipeline (dmg/nsis/appimage); Nix users — including the maintainer's own machines — have no `pkgs.waypoints`. Meanwhile `devenv.nix` has grown to ~380 lines, most of it transformation plumbing (mise projection, treefmt command rewriting, Android SDK assembly) that buries the actual configuration. This change addresses both at once: the plumbing moves into modules under `nix/`, and a flake-parts build layer exposes the desktop app through an overlay.

## What Changes

- Extract the three transformation concerns from `devenv.nix` into devenv modules under `nix/devenv/` (`mise.nix`, `treefmt.nix`, `android.nix`), leaving `devenv.nix` as imports + configuration. The devenv shell and every generated file (`.config/mise.toml`, `.config/treefmt.toml`, `local.properties`, `libs.versions.toml`) must remain **byte-identical**, gated by `just check-generated`.
- Add `flake.nix` (flake-parts) and `nix/overlay.nix` exposing the desktop app as installable packages: `waypoints` (stable), `waypoints-preview`, `waypoints-nightly`. The development flavor is deliberately not packaged.
- Build the Nuxt frontend as a separate, flavor-parameterized derivation whose output is embedded into the Tauri binary at compile time — the frontend remains a subpart of the desktop app, never a standalone artifact.
- Bypass Tauri's bundler: Nix assembles the platform wrapper itself (`Waypoints.app` directory on darwin, binary + `.desktop` + icons on Linux). Both macOS and Linux are supported targets.
- Nix-installed builds disable **in-place upgrade** (a store path cannot rewrite itself) but retain **update notifications**: the app still checks the flavor's update endpoint and surfaces new versions to the user.
- The devenv universe is untouched: `devenv.yaml`/`devenv.lock`/secretspec remain authoritative for the dev shell; the flake's inputs are independent and the overlay is nixpkgs-agnostic (no inputs imposed on consumers).

## Capabilities

None of the registered spec domains (data-model, editor, import, app-shell, storage, sync-protocol, sync) covers packaging or distribution. Rather than silently minting one, this proposal explicitly introduces a new domain: **`distribution`** — how Waypoints builds are packaged, installed, and receive updates outside the in-app bundler pipeline.

### New Capabilities

- `distribution/nix-overlay`: installing Waypoints via the Nix overlay — the package set and flavor mapping (stable/preview/nightly), platform wrapper contracts on darwin and Linux, frontend-embedding guarantee, and the notify-only update behavior of store-installed builds.

### Modified Capabilities

None. The devenv module extraction is a pure refactor — byte-identical generated output is the acceptance gate, so no spec-level behavior changes there.

## Impact

- **`devenv.nix`**: shrinks to configuration + `imports`; new `nix/devenv/{mise,treefmt,android}.nix` modules own their transformation logic and the generated files they feed.
- **New files**: `flake.nix`, `flake.lock`, `nix/overlay.nix`, `nix/packages/waypoints/{package,_frontend}.nix`.
- **`src-tauri`**: the updater path needs a build-time distinction so Nix builds check-and-notify without installing (exact mechanism is a design decision). The app manifest also becomes the single source of the packaged version, so the flavor configs drop their `version` fields (see design.md D9).
- **Version control**: `flake.lock`, `Cargo.lock`, and the pnpm fixed-output hash become committed artifacts. `Cargo.lock` must be tracked at all — the flake source omits gitignored files, so an untracked lock is invisible to the build; the FOD hash must track `pnpm-lock.yaml` changes.
- **No consumer-imposed dependencies**: the overlay uses only `final:`/`prev:`; the flake's own nixpkgs pin serves CI/local `nix build` only.

## Non-goals

- `waypoints-cli` packaging — the crate does not exist yet; it drops into the same overlay later.
- Settings/configuration management via home-manager or NixOS modules.
- Migrating devenv onto the flake — `devenv.yaml` stays; secretspec and dynamic-path ergonomics depend on it.
- Code signing, notarization, or dmg/nsis/appimage production via Nix — the existing release pipeline owns those distribution artifacts.
- Android/iOS builds via Nix.
- Making `nix build` the developer inner loop — `just` + cargo remains the iteration path.
- Binary cache (cachix) publishing for overlay consumers.
