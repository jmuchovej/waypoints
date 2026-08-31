# Tasks

## 1. Spikes (throwaway; results decide structure, code is discarded)

- [x] 1.1 Verify devenv 2.2.1 accepts `imports` in `devenv.nix` — add a trivial module (one env var), enter the shell, confirm the var is set; then remove it
- [x] 1.2 Verify devenv coexists with a root `flake.nix` — drop a minimal stub flake at repo root, run `devenv shell` and `just check-generated`, confirm zero behavior change; keep or delete stub per result
- [ ] 1.3 Spike the pnpm FOD: minimal derivation running `pnpm.fetchDeps`/`pnpmConfigHook` (pnpm 11) against `pnpm-lock.yaml`, verified by an offline `pnpm install --frozen-lockfile` succeeding inside the sandbox; record the working hook combination (or the fallback needed) in design.md

## 2. devenv module extraction (byte-identical gate)

- [x] 2.1 Capture the baseline: enter the current shell, commit/stash any pending generated-file drift so `just check-generated` passes clean before the refactor
- [x] 2.2 Extract `nix/devenv/treefmt.nix` (wrapper table, excludes, bare-command rewrite, `.config/treefmt.toml` files entry) behind `waypoints.treefmt.*` options; verify shell entry + `just check-generated` zero diff
- [x] 2.3 Extract `nix/devenv/mise.nix` (spec table, `unavailable` list, formatter projection, `mise-tools`, `.config/mise.toml`/`.config/mise.lock` files entries, `mise:lock` task) behind `waypoints.mise.*` options; verify shell entry + `just check-generated` zero diff
- [x] 2.4 Extract `nix/devenv/android.nix` (version spec, system-image expansion, SDK assembly, `ANDROID_*`/`GRADLE_OPTS`/`LD_LIBRARY_PATH`/`PATH` env, `local.properties` + `libs.versions.toml` files entries) behind `waypoints.android.*` options; verify shell entry + `just check-generated` zero diff
- [x] 2.5 Confirm `devenv.nix` is reduced to `imports` + configuration (no transformation helpers remain) and delete the stale `nix/mise.nix`/`nix/tools.nix` stubs; verify `just check-generated` and a full `just tauri::android::dev` toolchain resolution still work

## 3. Flake skeleton and overlay plumbing

- [ ] 3.1 Add `flake.nix` (flake-parts; nixpkgs = nixos-unstable) with `perSystem` for aarch64-darwin/x86_64-linux/aarch64-linux and top-level `flake.overlays.default = import ./nix/overlay.nix`; verify `nix flake show` lists the expected systems and `devenv shell` still enters cleanly
- [ ] 3.2 Add `nix/overlay.nix` as a pure `final: prev:` (optional `rustToolchain` arg per D3); verify it imports without flakes via `nix-instantiate --eval`
- [ ] 3.3 Prove the Rust wiring with a throwaway `nix build` of `tooling/xtask` using a `lib.fileset` src and `final.rustPlatform` + `importCargoLock`; verify the binary runs, then delete the throwaway package
- [ ] 3.4 Verify `GITHUB_SHA`-from-`self.rev` plumbing: sandbox build stamps a real sha on a clean tree and falls back gracefully on a dirty one (build.rs already handles absence)

## 4. Notify-only update path (`self-update` feature)

- [ ] 4.1 Add the `self-update` cargo feature to `src-tauri/Cargo.toml`, included in `default`; verify existing builds (feature on) are behavior-identical and hooks stay green
- [ ] 4.2 Gate `check_for_updates` in `src-tauri/src/builders/menus.rs`: feature off → notify (dialog with new version, no install action) and never call `download_and_install`; verify by compiling with `--no-default-features --features desktop,<flavor>` and exercising the menu path against a mock/staging endpoint, plus feature-on path unchanged
- [ ] 4.3 Verify the endpoint-unreachable scenario stays a logged non-event in both feature modes (spec: "Endpoint unreachable")

## 5. Frontend derivation

- [ ] 5.1 Implement `nix/packages/frontend.nix` (flavor arg; fileset over `src/`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, flavor configs; `VITE_WAYPOINTS_FLAVOR` from the flavor toml's identifier; `nuxi generate`), using the hook combination from 1.3; verify `nix build` of the stable flavor emits the static site and record which directory (`.output` vs `.output/public`) holds `index.html`
- [ ] 5.2 Build all three packaged flavors (stable/preview/nightly) of the frontend; verify each embeds its own `VITE_WAYPOINTS_FLAVOR` (grep the emitted assets for the flavor identifier)
- [ ] 5.3 Document the FOD hash update step (command + file location) as a comment where the hash lives; verify by intentionally perturbing the hash and confirming the error message points to the fix

## 6. Desktop app derivation and wrappers

- [ ] 6.1 Implement `nix/packages/waypoints.nix` core build: `cargo tauri build --no-bundle --features <flavor>` with platform + flavor `--config` layering, `beforeBuildCommand` neutralized, `frontendDist` pointed at the 5.1 output; verify the binary builds on the host platform and launches with the embedded UI
- [ ] 6.2 Assemble the darwin wrapper: `$out/Applications/<ProductName>.app` with `Contents/MacOS/` binary, generated `Info.plist` (flavor identifier + `waypoints://` CFBundleURLTypes), flavor `.icns`; verify the bundle launches from Finder after linking into `~/Applications`
- [ ] 6.3 Assemble the Linux wrapper: `$out/bin/waypoints`, generated `.desktop` entry, hicolor icons; verify on a Linux host/VM that the app launches from `$PATH` and appears in the launcher
- [ ] 6.4 Wire `nix/overlay.nix` and `perSystem` packages to expose `waypoints`, `waypoints-preview`, `waypoints-nightly` (no development flavor); verify `nix flake show` lists exactly these and `nix build .#waypoints-nightly` produces nightly branding
- [ ] 6.5 Verify overlay purity end-to-end: from a scratch flake with only nixpkgs + the Waypoints flake as inputs, apply `overlays.default` and build `pkgs.waypoints` (spec: "Consumer installs via their own nixpkgs")

## 7. Spec validation

- [ ] 7.1 Co-installation: install `waypoints` and `waypoints-nightly` into one profile; verify both launch independently under distinct product names/identifiers (spec: "Nightly and stable coexist")
- [ ] 7.2 Closure hygiene: `nix path-info -r` on each package; verify no nodejs/pnpm/frontend-source paths in the runtime closure (spec: "Closure hygiene")
- [ ] 7.3 Notify-only cycle: run a store-installed build against an endpoint advertising a newer version; verify notification appears, no write occurs to the install path, and the app keeps working with the endpoint blocked (spec: update requirements, all three scenarios)
- [ ] 7.4 Deep-link registration: with the darwin app linked into `~/Applications`, verify `open "waypoints://"` (or a pairing link) routes to the Nix-installed flavor (spec: "macOS install is LaunchServices-linkable")
- [ ] 7.5 Byte-identical closing check: `just check-generated` green on the final tree, confirming the devenv universe survived the whole change untouched
