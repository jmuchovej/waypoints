## Why

No release group's version is declared anywhere. It is whatever the group's anchor manifest's `[package].version` happens to say, and the lockstep check holds the other members to it — so every reader reconstructs "which manifest is the group's" for itself: the planners through `anchor_version` and `workspace::version()`, `src-tauri/build.rs` and the Nix app recipe through the group file's `anchor` and that manifest's `[package]`, `nix/channel.nix` through a hard-coded path to `src-tauri/Cargo.toml`. The group file names every group and cannot say what version any of them is. The `anchor` key exists only to point at a manifest, three library manifests were held to TOML 1.0 only so Nix could read them, and a series' bill of materials is stated nowhere a person reads — it is pins scattered through a lockfile.

Decided with John on 2026-11-17: every group's version — the four library groups and the app — is declared once, as a `version` key per group in `tooling/release-groups.toml`, and the member manifests become projections the release bot writes and a pull-request check holds in agreement. Cargo still needs a version in every manifest, so the manifests keep one; the file is where it is declared and read. This reverses `release-groups-guards`' D1 — versions live only in manifests, so the file never takes a bookkeeping commit — and design.md D1 says why that is now safe.

Why now: no cut has run under the current pipeline, so the first cut is the first that writes five groups' versions anywhere. Landing this before it means the first cut writes the file from the start and no series ever exists whose file says nothing. `library-minting` finished its mechanism on 2026-11-16 and left its three run-time audits open; this change lands before the first cut, folds those audits into its own last task group, and `library-minting` stays open until they run.

## What Changes

- **The file declares; manifests project.** Each `[group.<name>]` gains `version`. One projection routine writes a group's version into the file, every member manifest, and the lockfile entries together, and is the planner's only way of writing a version; `cargo xtask release-groups project` runs the same routine over a hand-edited file; `version-integrity` holds every member manifest to its group's declared version on every pull request. The bot's cut writes the file, every changed member manifest, and the lockfile in one commit on `main`; advance, promote, and patch write the app's version to the file and the app's manifests; `patch` also writes a moved pin's version to the file, so a series' file always states what the series ships.
- **`anchor` goes.** Lockstep makes every member equivalent, and the file now says the version. The pin a build reads from the lockfile is found by the tag's shape (`?tag=<prefix>…` in the source) rather than by a package name; the app group is the one that owns the remainder, and the app's own crate the package `xtask` already names.
- **Readers read the file.** The planners (each group's current version and the app's), `src-tauri/build.rs` (the declared fallback), the Nix app recipe (`group-versions` and `manifest-version`), and `nix/channel.nix` (the channel and series). Tauri keeps reading the projected manifest, as Cargo does. The group file's tombi directive moves from TOML 1.1 to 1.0, which `builtins.fromTOML` requires and the file already satisfies; the TOML 1.0 directives on `crates/{bedrock,domain,transload}/Cargo.toml` stay.
- **The app's version joins the file**, channel suffix included: `0.2.0-nightly` on `main`, `0.2.0-pre.N` and then `0.2.0` on a series.
- **`version-integrity`:** lockstep compares every workspace member's manifest to the file, and a group a series pins to its pin; the version-writes rule reserved to `wp-botswain[bot]` covers the file's `version` keys; the series-pin check is unchanged. Both planners refuse a tree whose manifests disagree with the file, as they refuse one the file does not describe.
- **Dry runs** render the file's write in `main.patch` and `series.patch` beside the manifests'.

## Capabilities

This change stays inside the `distribution` domain: where a version is declared and who writes it is release planning, the checks that hold the projections are release integrity, and what the overlay reads is Nix delivery.

### New Capabilities

None.

### Modified Capabilities

- `distribution/release-planning`: _The app manifest is the sole source of a build's version_ is **REMOVED** and replaced by the **ADDED** _The group file declares a build's version_, which carries both of its scenarios. _The manifest version names the channel_ and _Every workspace crate is named in the group file_ are **MODIFIED** — the anchor becomes the declared version — with every existing scenario kept and one added to the latter. Four requirements are **ADDED**: _A group's current version is the group file's declaration_, _A version write lands the group file and its projections together_, _A command projects the group file into the member manifests_, and _A dry run renders the group file's write_. The last stands beside `library-minting`'s MODIFIED _A dry-run transition reports its refs and renders its writes_ rather than modifying it again. `library-minting`'s delta is not edited; its one mention of the anchor manifest (_First cut of a group with no tag_) is superseded by _A group's current version is the group file's declaration_, and whichever of the two changes reaches the main spec second carries that sentence.
- `distribution/version-integrity`: _Group members declare their anchor's version_ is **REMOVED** and replaced by the **ADDED** _Group members declare their group's version_, carrying both of its scenarios; _A series' group file names its pins_ is **ADDED**; _Only the release bot writes versions_ is **MODIFIED**, both scenarios kept and two added.
- `distribution/nix-overlay`: _Flavor channels map to distinct co-installable packages_ is **MODIFIED** in the one sentence saying where a version is read from, all five scenarios kept. _Flake overlay exposes desktop packages_ is `library-minting`'s MODIFIED delta and is not touched: the Station still carries the version of the `outpost` manifest it builds, which in a tagged tree is the file's. _A series ref builds itself_ and _On `main`, each package builds its channel's published source_ stay as published: "the version its app manifest declares" remains true of a projection held equal to the file.

## Impact

- **Group file and schema:** `tooling/release-groups.toml` gains `version` per group and loses `anchor`; its directive becomes `#:tombi toml-version = "v1.0.0"`; `tooling/schemas/release-groups.schema.json` is regenerated.
- **`tooling/xtask`:** `release_groups.rs` (the parser, `version_of`, the app group by `owns = "remainder"`, `members_anchored_at` gone, `load` applying the agreement rule); `planners/release.rs` (`anchor_version`, `app_group`, `app_members`, `write_members` folded into the projection, `written_paths`, `Plan::landing` and `main_landing`, `execute`, `open_series`, the patch path, `rehearse` and `manifest_changes`); `version_integrity.rs` (`lockstep_violations`, `differing_versions` and `version_writes`); `workspace.rs` (`version()` goes); `semver_check.rs` (`library_members` leaves the app's crate out by name, not by anchor); `src/bin/xtask/tasks/release_groups.rs` (`project`), `publish_plan.rs`, and `release_stage.rs` (the app's version from the file); the group-file fixtures in `release_groups.rs`, `version_integrity.rs`, `releases/index.rs`, `tests/version_write.rs`, and `tests/planners_refuse.rs`.
- **Build data:** `src-tauri/build.rs` and `src-tauri/src/group_versions.rs`: the declared fallback is the file's, the pin is read by tag prefix, `Group` loses its anchor fields.
- **Nix:** `nix/lockfile.nix` answers by tag prefix as well as by package; `nix/packages/waypoints/package.nix` reads `group-versions` and `manifest-version` from the file; `nix/channel.nix` reads the app's version from the file.
- **Docs:** `docs/CONTRIBUTING.md`'s version-writes sentence names the file's `version` keys beside `[package].version`.
- **Contributor workflow:** a version is edited by hand nowhere. A new crate joining a group runs `cargo xtask release-groups project` so its manifest takes the group's version before its pull request; a hand-edited `version` fails `version-integrity` as a hand-edited manifest does.
- **Unchanged:** the generated workflows, the walk, what a cut mints, how a series pins, and `index::classify`'s tag-versus-tree check, which reads the tagged tree's app manifest — a projection held equal to the file.

## Non-goals

- No change to what a cut mints, how tags are minted, how a series pins, or what the walk counts; that is `library-minting`, which stays open until its run-time audits — folded into this change's last task group — have run.
- No Cargo workspace inheritance (`version.workspace = true`): it gives one version to the whole workspace, and five groups need five.
- No ledger file beside the group file; the file is the one declaration.
- No channel for libraries; a library group's `version` is bare.
- No removal of the TOML 1.0 directives on the three crate manifests; they stay.
- No change to `index::classify`, which keeps comparing a tag's version with the tagged tree's app manifest.
- No re-reading of history, and no retroactive file on a series: no series exists.
