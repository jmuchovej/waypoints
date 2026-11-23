# Tasks

## 1. The setting and the policy

- [x] 1.1 Add a `self_update` key (`off` | `notify` | `on`, default `on`) to the settings store in `src-tauri/src/settings.rs` and a `SelfUpdatePolicy` type with `Ord` so `min(setting, cap)` is a one-liner; verify unit tests cover the default when the key is absent and `min` over every pair
- [x] 1.2 Parse `--no-self-update` / `--self-update` in each `src-tauri/src/bin/*.rs` before Tauri starts: strip every occurrence, keep the last, leave `--version`'s sole-argument rule and every other argument untouched, and pass the cap into `waypoints::run`; verify unit tests cover `[--no-self-update]`, `[--no-self-update, waypoints://pair/x]`, `[--no-self-update, --self-update]`, and `[--version]` alone
- [x] 1.3 Record the cap once in the running instance so arguments forwarded by the single-instance plugin cannot change it; verify a test that forwards `[]` to a capped instance leaves it capped

## 2. One binary

- [x] 2.1 Rewrite `check_for_updates` in `src-tauri/src/builders/menus.rs` as one `match` on the effective policy (`off` returns before the check; `notify` shows the existing dialog; `on` downloads, installs, restarts) and delete both `#[cfg(feature = "self-update")]` arms; verify `cargo clippy -p waypoints --all-targets` is clean and a test over a fake updater asserts `off` never calls `check`
- [x] 2.2 Remove `self-update` from `[features]` and `default` in `src-tauri/Cargo.toml`; verify `cargo build -p waypoints` succeeds and `rg -n "self-update" src-tauri/Cargo.toml` is empty
- [x] 2.3 In the Waypoints Nix recipe (`distribution/nix/packages/waypoints/package.nix`), reduce `buildFeatures` to the flavor and add `--add-flags --no-self-update` to the wrapper; verify `nix eval --raw .#checks.aarch64-darwin.waypoints-in-tree.drvPath` succeeds and the built wrapper script contains the flag

## 3. The settings page

- [x] 3.1 Add a command returning `{ setting, cap, effective }` and one setting `setting`, regenerate bindings with `cargo xtask typegen`; verify `just check-generated` passes once committed
- [ ] 3.2 Add the self-update control to the app settings section (beside `src/app/pages/settings/app/appearance.vue`): three options, values above `cap` disabled, the notice from design D4 with its link shown when `cap` is below `on`; verify a component test renders all three enabled without a cap and `on` disabled with the notice under a cap
- [ ] 3.3 Add the managed-installs docs page to the website at the URL the notice links to, listing Nix today and saying the AUR joins when its package ships; verify the page builds and the URL in the app matches it

## 4. Specs and integration

- [x] 4.1 Confirm the `nix-overlay` delta quotes the published requirement in full with its scenarios; verify `openspec validate self-update-policy --strict` passes
- [ ] 4.2 Run `jj fix -s @`, the hook steps, `cargo nextest run -p waypoints`, and `just check-generated`; verify all green
