## Context

See proposal.md for motivation. Current state that shapes the approach:

- `devenv.nix` (~380 lines) mixes three transformation concerns — mise projection, treefmt command rewriting, Android SDK assembly — with genuine configuration. Each transformation feeds generated files (`.config/mise.toml`, `.config/treefmt.toml`, `local.properties`, `libs.versions.toml`) already guarded by `just check-generated`.
- No `flake.nix` exists; devenv 2.2.1 is driven by `devenv.yaml`/`devenv.lock`. The repo is jj-colocated, so the git index lags the working copy — `self`-based flake sources are snapshot-timing-sensitive.
- Flavors are already first-class: cargo features `development`/`nightly`/`preview`/`stable` on the `Waypoints` crate (forwarding to `store/<flavor>`), Tauri config layering via `--config configs/platforms/<os>.toml --config configs/flavors/<flavor>.toml`, and `VITE_WAYPOINTS_FLAVOR` derived from the flavor's identifier for the frontend build.
- The frontend is an SSR-off Nuxt SPA (`nuxi generate`), embedded into the binary at compile time via `frontendDist` — verified: nothing ships standalone.
- The updater currently does check → `download_and_install` → restart in `src-tauri/src/builders/menus.rs::check_for_updates`, triggered from a menu item. `tauri-plugin-dialog` is already a dependency.
- `src-tauri/build.rs` stamps build metadata from `GITHUB_SHA` or `git rev-parse` (falls back to `"unknown"`).

## Goals / Non-Goals

**Goals:**

- Byte-identical devenv outputs across the refactor — provable, not asserted.
- The two Nix universes (devenv shell, flake build layer) stay decoupled: independent inputs, independent locks, no shared modules.
- Flavor parity with the existing build pipeline: the Nix build reuses the same feature flags, config layering, and frontend flavor injection rather than inventing a parallel mechanism.
- The bundler-based release pipeline is untouched and remains the default self-updating distribution.

**Non-Goals (design-level, beyond proposal's):**

- No sharing of `nix/devenv/` modules with the flake — dependency stays one-directional.
- No attempt at bit-reproducible binaries (build.rs stamps a wall-clock timestamp; accepted).
- No `nix flake check`-driven CI gating in this change (deferrable, see Open Questions).

## Decisions

### D1 (2026-08-30): devenv transformations become devenv modules, not plain functions

`nix/devenv/{mise,treefmt,android}.nix` are devenv modules listed in `devenv.nix`'s `imports`, exposing options under a single `waypoints.*` namespace (e.g. `waypoints.android.ndk.version`, `waypoints.mise.specs`, `waypoints.mise.unavailable`, `waypoints.treefmt.excludes`). Each module owns the `files` entries it generates — no hoisted shared `files` block.

- **Why**: `mise.nix` reads `config.treefmt.config.programs` _and_ `config.languages.*.package`; the module fixpoint resolves that ordering for free. devenv is already a NixOS-style module system — this is the boring option.
- **Rejected**: plain imported functions — devenv.nix keeps the wiring (~120 of ~230 lines saved instead of nearly all), and call ordering between the three concerns becomes manual.
- Only _data_ becomes options (versions, spec tables, exclude lists, wrapper maps); helpers like `dash-version` stay module-private.
- **Gate**: re-enter the shell, `just check-generated` passes with zero diff — byte-identical generated files.

### D2 (2026-08-30): The flake is packaging-only; devenv stays on devenv.yaml (user decision)

flake-parts structures the build side only. `devenv.yaml`/`devenv.lock`/secretspec remain authoritative for the dev shell — flake-based devenv drops secretspec support and pins things like `config.git.root` per-system.

- The flake's own nixpkgs tracks `nixos-unstable` (not `devenv-nixpkgs/rolling`, which exists for devenv's compat needs) so CI builds resemble consumer environments.
- Two lockfiles are deliberate, not drift to fix; Renovate manages both.
- `nix develop` is not a supported entry point; `devenv shell` remains the only shell.

### D3 (2026-08-30): Overlay is pure and nixpkgs-agnostic

`nix/overlay.nix` is a plain `final: prev:` that `callPackage`s `nix/packages/*` — importable without flakes. `flake.nix` reduces to `flake.overlays.default = import ./nix/overlay.nix;` (top-level, system-agnostic) plus `perSystem` `packages`/`checks`.

- Rust toolchain: `final.rustPlatform` from the consumer's nixpkgs. `rust-toolchain.toml` pins no exact version (`channel = "stable"`, `rust-version = "1.91"`), so a minimum-nixpkgs note suffices.
- **Escape hatch**: the overlay file takes an optional `rustToolchain` argument for pinning without changing the default.
- **Rejected**: requiring consumers to carry rust-overlay — imposes our input graph, defeats the point of shipping an overlay.

### D4 (2026-08-30): Package sources via `lib.fileset`, never `self`

Each derivation in `nix/packages/` builds its `src` with `lib.fileset.toSource`, enumerating exactly the workspace paths it needs.

- **Why**: jj colocation makes "what does git see right now" a snapshot-timing question — flake `self` silently omits files git hasn't been told about. Explicit filesets are deterministic regardless of git/jj state, keep source copies small (repo carries `target/`, `node_modules/`, a 296-entry `.devenv/`), and stop `website/` edits from invalidating app builds.

### D5 (2026-08-30): Frontend as a flavor-parameterized derivation (pnpm FOD)

`nix/packages/frontend.nix` takes `flavor`, runs `nuxi generate` with `VITE_WAYPOINTS_FLAVOR` set from the flavor config's identifier (mirroring `src/justfile`), using nixpkgs' pnpm fetch/config hooks against `pnpm-lock.yaml`.

- The output is a build-graph intermediate only — consumed by D6, never exposed as a package.
- **Risk owner** for the change's highest-friction step: nixpkgs pnpm-FOD support for `pnpm_11` must be spiked before dependent work (see Risks).
- The exact directory handed over (`.output` vs `.output/public` — `Tauri.toml` says the former, `nuxi generate` emits the site under the latter) is resolved at implementation: whichever contains `index.html` wins.

### D6 (2026-08-30): Build via `cargo tauri build --no-bundle`; Nix assembles the wrappers

The app derivation runs the existing pipeline minus bundling: `cargo tauri build --no-bundle --features <flavor> --config configs/platforms/<os>.toml --config configs/flavors/<flavor>.toml`, with `beforeBuildCommand` neutralized by a config override and `frontendDist` pointed at the D5 output. `pkgs.cargo-tauri` already exists (it's in the dev shell's mise tools).

- **Why not plain `cargo build`**: the `--config` layering semantics live in the tauri CLI; replicating the merge by hand-rolling `TAURI_CONFIG` JSON duplicates behavior that must never drift from the justfile pipeline.
- **Why not `cargo tauri build` with bundles**: codesign/notarization/`hdiutil` are sandbox-hostile on darwin, and dmg/nsis/appimage are distribution formats owned by the existing release pipeline.
- Wrappers, per platform:
  - **darwin** (aarch64 only — Intel macOS is EOL Sep 2026 per `rust-toolchain.toml`): assemble `$out/Applications/<ProductName>.app` — binary, generated `Info.plist` carrying the flavor's identifier and the `waypoints://` scheme (CFBundleURLTypes), flavor `.icns` from `src-tauri/icons/<flavor>/`. Unsigned is fine: locally built store paths carry no quarantine xattr. LaunchServices registration (deep-link pairing depends on it) comes from app-linking into `~/Applications` — spec requirement, verified in validation.
  - **Linux** (x86_64, aarch64): `$out/bin/waypoints`, generated `.desktop` entry, hicolor icons from the flavor set.
- Build metadata: derivations set `GITHUB_SHA` from `self.rev or self.dirtyRev or "unknown"` since the fileset src contains no `.git`. The build.rs wall-clock timestamp is accepted nondeterminism (binary metadata only).

### D7 (2026-08-30): Notify-only updates via a `self-update` cargo feature

Add a `self-update` feature to the `Waypoints` crate, included in `default`. `check_for_updates` keeps today's behavior when the feature is on; when off, the `Ok(Some(update))` arm surfaces a notification (dialog via the already-present `tauri-plugin-dialog`, or native notification) with the new version — and never calls `download_and_install`. Nix builds compile with `self-update` off.

- **Why a feature**: flavors are already cargo features on this crate; this matches the established mechanism, is visible in the feature matrix, and keeps the bundler pipeline's behavior unchanged (additive default).
- **Rejected**: compile-time env marker (`option_env!`) — a second, less discoverable configuration channel alongside existing features; runtime `/nix/store` path detection — implicit and clever where explicit and boring works, and false behavior if the bundle is copied out of the store.
- The updater _plugin_ stays in the build in both modes — `check()` is what queries the flavor endpoint; only the install path is gated. Endpoint-unreachable stays a logged non-event (already the case).

### D8 (2026-08-30): Layout

```
flake.nix                      # flake-parts: inputs, perSystem packages/checks, overlays.default
devenv.nix                     # imports + configuration only
nix/
  devenv/                      # devenv module functions (dev-shell universe)
    mise.nix
    treefmt.nix
    android.nix
  overlay.nix                  # final: prev: — pure, flake-independent
  packages/                    # callPackage derivations (build universe)
    frontend.nix
    waypoints.nix
```

The `devenv/` vs `packages/` split is load-bearing: two universes, two input graphs, two calling conventions. `overlay.nix` is the boundary and the only file that must import cleanly without flakes.

## Risks / Trade-offs

- **[pnpm_11 FOD support in nixpkgs may lag]** → Spike `pnpm.fetchDeps`/`pnpmConfigHook` against `pnpm-lock.yaml` _before_ building anything downstream of D5. If broken: fall back to `pnpm_10`-driven fetch of the same lockfile if compatible, else a hand-rolled `pnpm fetch` FOD. This is the step most likely to reorder the plan.
- **[devenv 2.2.1 may not tolerate a root `flake.nix`, or `imports` in devenv.nix]** → Spike both with throwaway files before committing to the structure. `.gitignore` already carries `.devenv.flake.nix` (devenv's own shim — different filename, no clobber expected).
- **[pnpm FOD hash goes stale on every `pnpm-lock.yaml` change]** → Committed hash must be updated alongside lockfile changes; document the update command where the hash lives. CI enforcement is an open question.
- **[Byte-identical gate could mask ordering-dependent output]** → `treefmt.toml`/`mise.toml` generation serializes attrsets (sorted by Nix), so ordering is stable; the gate diff catches any surprise anyway.
- **[Unsigned darwin bundle]** → Fine for self-installed use (no quarantine xattr on local builds); any future distributable signing happens outside the derivation. Team ID–gated entitlements are out of scope.
- **[`cargo tauri build` inside the sandbox may attempt network or tooling access beyond cargo]** → `--no-bundle` with `beforeBuildCommand` neutralized should reduce it to a cargo invocation; verify during implementation, fall back to plain `cargo build` + `TAURI_CONFIG` merge only if the CLI proves sandbox-incompatible (accepting the drift risk documented in D6).
- **[Rebuild cost: no incremental compilation under `nix build`]** → Accepted; `just` + cargo stays the inner loop (explicit non-goal).

## Migration Plan

1. **Spikes** (throwaway, parallel): devenv `imports` + root-flake coexistence; pnpm_11 FOD viability.
2. **devenv extraction** (D1): move the three concerns, land only when `just check-generated` shows zero diff.
3. **Flake skeleton** (D2/D3/D8) + a throwaway `nix build` of an existing pure-Rust workspace member (e.g. `tooling/xtask`) to validate fileset/cargo/toolchain wiring; delete the throwaway target after.
4. **`self-update` feature** (D7) in `src-tauri` — independently landable and testable via the bundler pipeline (feature on = no behavior change).
5. **`frontend.nix`** (D5), proven standalone per flavor.
6. **`waypoints.nix`** (D6) consuming 4+5; darwin and Linux wrappers; wire `overlay.nix` and `perSystem` packages for all three flavors.
7. **Validation** against the spec scenarios: co-installation, closure hygiene, notify-only cycle, LaunchServices deep-link registration via app-linking.

Rollback: the flake layer is purely additive (delete `flake.nix`/`nix/overlay.nix`/`nix/packages/`). The devenv extraction reverts by restoring the previous `devenv.nix`; the byte-identical gate means any regression is caught at the first shell entry, not in production.

## Open Questions

- Which directory `nuxi generate` output hands to `frontendDist` (`.output` vs `.output/public`) — resolved by inspection at implementation; does not affect specs or task breakdown.
- Whether CI should build the Nix packages (and enforce FOD hash freshness) — deferrable; the PR CI workspace is a separate effort.
- Whether the notify-only dialog should link users to update instructions (e.g. "update via your Nix profile") — UX copy decision, safely made during implementation.
