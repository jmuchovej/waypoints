# Proposal

## Why

Whether a Waypoints install may rewrite itself is decided at compile time today: the `self-update` cargo feature is on for every published artifact and off only for the Nix build, which `package.nix` compiles without it. That gives two binaries for one release and leaves no way to ship a package-manager-owned install from an existing artifact — the AUR package, which repackages the published deb, would self-update over files pacman owns. The decision belongs at launch, where the package that installed the app can state it, and the user's own preference belongs in settings.

## What Changes

- The app gains a setting, `self_update`, with three values: **off** (never check), **notify** (check, and tell the user when a newer version exists), **on** (check and apply). The default is on. It is stored with the other app settings and shown on the settings page.
- The app accepts a launch flag, `--no-self-update` (with `--self-update` as its explicit inverse), that caps the effective policy at notify without writing the stored setting. On the settings page a capped install shows "on" disabled with a notice: _Self-updating is disabled because this copy of Waypoints is managed by a package manager, which delivers updates itself. Learn more._ — linking to a page on the website that names the managed channels.
- **BREAKING** (build surface): the `self-update` cargo feature is removed. One binary serves every channel; `check_for_updates` branches on the effective policy at runtime.
- The Nix recipe stops disabling the feature and instead wraps the binary with `--no-self-update`, so a Nix-installed build behaves as before through the launcher rather than the compiler.
- A docs page on the website explains managed installs and lists the channels that launch this way (Nix today; the AUR when `distribution-tree` lands its package).

## Capabilities

### New Capabilities

- `app-shell`: the desktop app's runtime lifecycle as the shell owns it — here, how the app decides whether to check for, announce, or apply an update: the setting, the launch flag that caps it, and what a capped install tells the user. This is the first spec in the `app-shell` domain; it owns only what this change specifies.

### Modified Capabilities

- `distribution/nix-overlay`: the requirement "Store-installed builds notify about updates but never self-update" moves from a build-time guarantee (a binary compiled without the ability) to a launch-time one (the package's wrapper passes `--no-self-update`), keeping every scenario.

## Non-goals

- Any package other than Nix passing the flag. `distribution-tree` adds the AUR package that does; snap and Flathub would, if they are ever added.
- Changing how or when the update check runs, the updater endpoints, or the artifact formats the updater accepts.
- A per-manager message. The notice is generic; the linked page carries the list.
- Exposing the policy to the frontend beyond what the settings page needs (the effective value and whether it is capped).

## Impact

- **App**: `src-tauri/src/bin/{waypoints,waypoints-preview,waypoints-nightly,waypoints-development}.rs` (argv handling, beside the existing `--version`), `src-tauri/src/lib.rs` (the policy handed to `run`), `src-tauri/src/builders/menus.rs` (`check_for_updates` branches at runtime; the `#[cfg(feature = "self-update")]` arms go), `src-tauri/src/settings.rs` (the `self_update` key), a command exposing the effective policy and the cap; `src-tauri/Cargo.toml` (the `self-update` feature removed from `[features]` and `default`); generated bindings.
- **Frontend**: a self-update control on the settings page under the app section (beside `src/app/pages/settings/app/appearance.vue`), disabled-with-notice when capped.
- **Nix**: `nix/packages/waypoints/package.nix` (`distribution/nix/packages/waypoints/package.nix` once `distribution-tree` lands): `buildNoDefaultFeatures`/`buildFeatures` reduced to the flavor, and `--add-flags --no-self-update` on the wrapper.
- **Website**: a docs page for managed installs, at a stable URL the app's notice links to.
- **Specs**: `openspec/specs/distribution/nix-overlay/spec.md` amended; `openspec/specs/app-shell/spec.md` created.
- **Depended on by**: `distribution-tree` (its AUR package passes the flag).
