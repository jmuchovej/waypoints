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
- **Amendment (2026-09-26)**: filesets select paths; they do not escape git filtering, so D4's headline ("never `self`") overstates the protection. The packages take a single `root` parameter (`nix/overlay.nix`, defaulting to `../.`), which under `nix build .#` resolves to the flake source — and that source _is_ git-filtered, the exact exposure D4 set out to avoid. It surfaced as a build failure: `Cargo.lock` was gitignored (the gitignore.io Rust template's leave-it-for-libraries default), so the flake source omitted it and `cargoLock.lockFile` died with `No such file or directory`. Resolved by committing `Cargo.lock` — correct for a workspace that ships executables, and already anticipated by proposal.md's note that the lock becomes a committed artifact. **The operative rule: every path a fileset names must be git-tracked, or the flake build cannot see it.**
- **Amendment (2026-09-26)**: `root` must be a real Nix _path_. `lib.fileset` rejects string-encoded paths and flake-input attrsets outright (`Paths represented as strings are not supported by lib.fileset`), so paths are built by concatenation (`root + "/Cargo.toml"`) against a path-valued `root` — never interpolation (`"${root}/Cargo.toml"`), and never `inputs.self`, which coerces to a store-path string.

### D5 (2026-08-30): Frontend as a flavor-parameterized derivation (pnpm FOD)

`nix/packages/frontend.nix` takes `flavor`, runs `nuxi generate` with `VITE_WAYPOINTS_FLAVOR` set from the flavor config's identifier (mirroring `src/justfile`), using nixpkgs' pnpm fetch/config hooks against `pnpm-lock.yaml`.

- The output is a build-graph intermediate only — consumed by D6, never exposed as a package.
- **Risk owner** for the change's highest-friction step: nixpkgs pnpm-FOD support for `pnpm_11` must be spiked before dependent work (see Risks).
- The exact directory handed over (`.output` vs `.output/public` — `Tauri.toml` says the former, `nuxi generate` emits the site under the latter) is resolved at implementation: whichever contains `index.html` wins.
- **Spike result (1.3, 2026-08-31)** — the working hook combination on nixos-unstable (nixpkgs d2f6794):
  - `pkgs.fetchPnpmDeps { pnpm = pkgs.pnpm_11; fetcherVersion = 4; ... }` + `pkgs.pnpmConfigHook` (top-level attrs; the `pnpm_11.fetchDeps`/`pnpm_11.configHook` package attrs are deprecated, and `fetcherVersion <= 3` is rejected for pnpm 11).
  - `package.json`'s `devEngines` (node ^24.4, deno ^2.9.4, `onFail: error`) is enforced at every pnpm boot — including inside the fetch sandbox, before install flags apply. The fetcher therefore needs `nativeBuildInputs = [deno]` (appended, not clobbered, by `fetchPnpmDeps`); node rides in via pnpm's runtime. The outer derivation needs `nodejs` + `deno` for the same reason. nixpkgs deno 2.9.5 satisfies the bound.
  - The FOD `src` fileset must carry: root `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml` (it holds `patchedDependencies`), `patches/`, and every workspace member's `package.json`.
  - Verified fetch hash (whole workspace, `--force` all-platform): `sha256-zFz9iUIhZQ66U5lSjCQ+RKiAXfUC7HcWuEAMd2CKgcU=`. Still current as of 2026-09-26.
- **Failure mode (2026-09-26)**: editing a file under `patches/` without reinstalling desyncs it from `pnpm-lock.yaml`, which records patch hashes under `patchedDependencies`. The FOD then fails `--frozen-lockfile` with `ERR_PNPM_LOCKFILE_CONFIG_MISMATCH`, _not_ a hash mismatch — the fix is `pnpm install` to refresh the lockfile, not a new FOD hash. Confirmed by doing it: the refreshed lockfile produced a byte-identical dependency set, so the recorded hash above was unchanged. Worth knowing because the error text points at the lockfile while the hash comment points at the FOD.
- **Finding (5.2, 2026-08-31)**: `VITE_WAYPOINTS_FLAVOR`'s only consumer is the dev-only `httpTransport` (`src/app/utils/ipc/invoke.ts`), which Vite dead-code-eliminates from production builds — the identifier cannot appear in packaged assets, for any flavor. Per-flavor frontend builds are kept anyway (user decision, 2026-08-31): the parameterization mirrors `src/justfile`, and any future production consumer of the var lights up correctly. Verification is a build-time assertion (`nix log` shows the injected identifier) instead of asset grep.
- **Deviation from D6's mechanism (2026-08-31)**: implementation uses nixpkgs' `cargo-tauri.hook` with the platform's bare bundle type (`--bundles app` on darwin, `--bundles deb` unpacked to `$out` on Linux) instead of `--no-bundle` + hand-assembled wrappers. D6 rejected "bundles" over dmg/nsis sandbox-hostility — the bare app/deb types involve none of that (no codesign/notarize/hdiutil), and tauri itself generates the Info.plist (identifier, URL schemes), `.desktop` entry, and icons from the same layered config the release pipeline uses, which is less drift-prone than replicating them in Nix. The spec's wrapper-layout requirements are unchanged; `beforeBuildCommand` is neutralized and `frontendDist` store-pointed via a third `--config` layer, exactly as designed. — Rust wiring proven by a throwaway `tooling/xtask` build: `lib.fileset` src (root `Cargo.toml`/`Cargo.lock`, `crates/`, `tooling/`, `vendor/`, `src-tauri/{Cargo.toml,build.rs,src}` — cargo demands every workspace member's manifest and target sources), `rustPlatform.buildRustPackage` with `cargoLock.lockFile` + `outputHashes."gh-workflow-0.9.0" = "sha256-CMzElgOmzseY3JOw7g2Q9uDGrS3usCzzdQt/nVNcn+E="` (the one git dependency), `cargoBuildFlags = ["--package" "<member>"]`. The `[patch.crates-io]` path-forks under `vendor/` resolve in-tree with no extra plumbing.

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

### D9 (2026-09-26): Version comes from the app manifest, not the flavor configs

`src-tauri/Cargo.toml`'s `[package] version` is the single source of truth, tracking the version of the branch it sits on (`main` and `v0.2.x` at `0.2.0`, `v0.1.x` at `0.1.0`). Flavor configs carry channel _identity_ — product name, bundle identifier, icons, update endpoint — and MUST NOT declare `version`.

- **Why**: a Tauri config `version` overrides the Cargo.toml fallback, so the four flavor configs' hardcoded `version = "0.1.0"` pinned every branch's bundle to `0.1.0` regardless of what the manifest said. Nothing maintained those fields either — `configs/flavors` has zero references anywhere in `tooling/` or `.github/`. Observed directly: a build stamped `CFBundleShortVersionString = 0.1.0` while cargo compiled `Waypoints v0.0.0`.
- Release tooling already treats the manifest as authoritative — `release_plan.rs`'s `APP_MANIFEST`, `set_package_version`, and `workspace::version` all read it, and `workspace.rs` carries a test asserting the app declares its own version rather than inheriting one.
- **Consequence**: the frontend derivation takes `version` as a parameter from the app derivation rather than parsing a manifest of its own. One parse, one source.

### D10 (2026-09-26): Manifests Nix parses stay TOML 1.0

`builtins.fromTOML` is a strict TOML **1.0** parser. The tree standardizes on TOML **1.1** via tombi (`#:tombi toml-version = "v1.1.0"` in `.config/treefmt.toml` and elsewhere), which permits multi-line inline tables — `src-tauri/Cargo.toml`'s `rustls` dependency is written as one and is unparsable by Nix, although cargo's lenient `toml_edit` accepts it (`cargo metadata` exits 0).

- Any manifest the Nix build reads is therefore pinned to TOML 1.0 (`#:tombi toml-version = "v1.0.0"`, as `.config/clippy.toml` already does) and kept free of 1.1-only constructs.
- **Rejected**: reformatting the offending entry without pinning the version — tombi would be free to re-expand it and nothing would warn.
- **Rejected**: reading the version from the flavor configs instead because they happen to be 1.0-clean — that is precisely the drift D9 removes.

### D8 (2026-08-30): Layout

```
flake.nix                      # flake-parts: inputs, perSystem packages/checks, overlays.default
devenv.nix                     # imports + configuration only
nix/
  devenv/                      # devenv module functions (dev-shell universe)
    mise.nix
    treefmt.nix
    android.nix
  overlay.nix                  # final: prev: — pure; takes `root` (path) and optional `rustToolchain`
  packages/                    # callPackage derivations (build universe)
    waypoints/
      package.nix              # the desktop app, one flavor per call
      _frontend.nix            # build-graph intermediate; underscore = never a package
```

The `devenv/` vs `packages/` split is load-bearing: two universes, two input graphs, two calling conventions. `overlay.nix` is the boundary and the only file that must import cleanly without flakes.

- **Amendment (2026-09-26)**: the packages live one directory deeper than planned (`packages/waypoints/{package,_frontend}.nix`, not `packages/{waypoints,frontend}.nix`) so each package owns a directory and its intermediates; the `_` prefix marks a file that is never exposed as a package. Noted in task 5.1 when it landed, recorded here for the layout of record.
- **Amendment (2026-09-26)**: "the only file that must import cleanly without flakes" is a comment in `overlay.nix`, not a requirement — spec.md asks only that the overlay be self-contained (no _extra_ consumer inputs), and consumers are expected to add the Waypoints flake itself. Flake-free import does work and was verified, but nothing tests it; treat it as a property, not a contract.

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
- **Published-release fidelity — deferred to its own change (2026-09-26).** This change builds all three flavors from whichever ref the consumer pins, so of the nine flavor×ref combinations only three are meaningful: the rest produce artifacts that misrepresent their channel (a stable-branded, `so.waypoints`-identified build of `main`'s code, which would also collide with a real stable install). Nightly is the sharpest case: `nightly_plan` computes `-nightly.N+YYYY.MM.DD` and emits it as a CI plan without ever committing it, and flake metadata exposes `self.rev` but never the tag name — so a nightly's version is unrecoverable from any tree at any ref, and a Nix build on 2027-05-17 cannot reproduce the 2027-05-15 nightly's stamp. Two directions were explored:
  - **Ref selects the channel**: version from the manifest (D9), flavor derived from it (`channel_for` already does most of this, though it classifies a nightly prerelease as preview). One package attribute, which contradicts this spec's three-attribute requirement. Cannot reproduce published nightly versions.
  - **Committed release index**: `nix/releases/{stable,preview,nightly}.json` on `main`, each recording the published tag plus rev, source hash, and pnpm FOD hash; packages `fetchFromGitHub` their tag and take the version from it. Preserves nightly dates exactly, keeps the three-attribute surface, and works for tags on release branches because tags are repo-global and the index is metadata rather than code. Costs: cross-branch writes from release CI, two hashes captured post-tag (the pnpm hash _must_ be recorded — `fetchFromGitHub` yields a store path at build time while `fetchPnpmDeps` needs its hash at eval time), a bootstrap gap for pre-index releases, and a one-line change to `nightly_plan`'s `commits_since` to exclude the index path — otherwise every index commit manufactures the justification for the next nightly and the quiet-repo skip is destroyed.
