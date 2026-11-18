# Proposal

## Why

Waypoints reaches users today through two channels — the direct download the website serves from the release store, and the Nix overlay — and the tree keeps what it has of distribution in two places that describe neither: the Station's Kubernetes manifests under `deploy/`, and the overlay under `nix/` beside the dev shell's modules, which have nothing to do with shipping. Package-manager channels (Homebrew, the AUR, winget, Chocolatey) each want a manifest that names the same release the store already publishes; written by hand, that is one file per channel per release channel — twelve files differing only in identities, versions, and digests — and they drift. This change gives distribution one tree, one registry of which distribution channels each release channel reaches, and one contract for how a channel's manifest derives from the published release.

Terms used throughout: a **release channel** is stable, preview, or nightly, and shares its name with the app **flavor** built for it. A **distribution channel** is a package manager or deployment target. A **package manifest** is the template a distribution channel keeps in the tree; a **flavored package manifest** is that template rendered for one release channel; the **release manifest** is the `{channel}.json` record the publishing run writes to the store.

Tree changes below are stated against commit `ce4067aca`. `main` has moved since (nightly tags are date-named, `v0.1.0-nightly.20261116`, with no build metadata); the change is rebased before implementation, and the lists of open changes and code references are re-derived then.

## What Changes

- **BREAKING** (task-runner surface): `deploy/` is removed and its contents move into a new top-level `distribution/` tree. The root `justfile`'s `mod deploy "deploy/justfile"` becomes `mod dist "distribution/justfile"`, so `just deploy::<recipe>` becomes `just dist::kubernetes::<recipe>`. The mise tool lane `deploy` (kubeconform, kubectl, kind) is renamed `dist`.
- The Kubernetes tree moves whole: `deploy/kubernetes/{base,kind}/*` → `distribution/kubernetes/{base,kind}/*`, `deploy/justfile` → `distribution/kubernetes/justfile`. Recipe names are unchanged (`lint`, `image`, `kind-up`, `kind-load`, `kind-apply`, `kind-down`).
- **BREAKING** (flake consumers, of which there are none yet): the Nix channel moves under the same tree — `nix/overlay.nix`, `nix/channel.nix`, `nix/packages/`, and `nix/releases/` → `distribution/nix/`. The root `nix-pins` recipes move with it, as `just dist::nix::pins` and `pins-check`. The overlay imports a recorded release's recipe from the tree at that release's revision, so `waypoints.nightly` on `main` fails to evaluate until the next nightly records a post-move revision (design D3).
- `nix/devenv/` (the dev shell's modules and the scripts devenv runs as tasks) moves to `tooling/devenv/`, beside the rest of how the repository builds itself; `nix/` is removed. `nix/AGENTS.md` moves with it and is trimmed to the devenv rules.
- `distribution/` adopts one contract: one directory per distribution channel, named for it; a channel's inner layout follows its own tooling's idiom (kustomize's base and overlays, Nix's recipes and release index) and never spills into a second top-level directory; a channel with validation recipes has a `justfile` at its root, and `distribution/justfile` declares a `mod` per such channel. Package channels hold package manifests — templates — never a flavored one.
- `distribution/channels.toml` is the registry: one table per release channel (`stable`, `preview`, `nightly`), each entry naming a distribution channel and the package identity that release channel takes there. It is both the renderer's source of identities and the record of which distribution channels serve which release channel.
- Four new channel directories: `brew/` (one Homebrew cask template, installing the dmg on macOS and the AppImage — with its desktop entry — on Linux), `aur/` (`-bin` PKGBUILD and `.SRCINFO`, from the deb), `winget/` (the three-manifest set), `chocolatey/` (nuspec and install script). Every release channel is registered on all four.
- The AUR package launches the app with self-updating disabled, through a `/usr/bin` wrapper and its desktop entry, because pacman owns the installed files. The flag it passes, `--no-self-update`, is defined by the `self-update-policy` change, which this change depends on for its AUR group.
- `cargo xtask dist render <channel> --flavor <flavor>` renders a distribution channel's package manifests — file names and contents — into flavored package manifests under `target/dist/`, from the release record: the identity from the registry, the product name and bundle identifier from `src-tauri/res/configs/flavors/<flavor>.toml`, the executable name from xtask, the version and series from the release manifest, download URLs from the release-delivery layout, and artifact digests from the release manifest.
- The release manifest gains a SHA-256 digest per uploaded file, and the download surface serves it at `/api/v1/releases/{channel}/manifest`, so a flavored package manifest's checksums derive from the record without a store credential or a re-download.
- The `Tests` workflow gains a `dist` job on the macOS runner that renders every channel for every registered release channel against a fixture and runs each channel's check; `brew audit` runs there, the Windows and Arch checks report skipped. It is not a required check.

## Capabilities

### New Capabilities

- `distribution/package-channels`: the package manifests through which third-party package managers install Waypoints — where they live, that they are templates deriving every release-specific value from the release manifest, the registry of which distribution channels each release channel reaches and as what, how a package-manager-owned install is launched, and how a flavored package manifest is validated before it leaves the tree.

### Modified Capabilities

- `distribution/station-deployment`: the requirement "Reference manifests describe a one-Station Fleet that survives pod replacement" names `deploy/kubernetes/` as the manifests' home; it becomes `distribution/kubernetes/`, base and kind overlay nested as before.
- `distribution/release-delivery`: the requirement "A channel's manifest is what an updater reads" gains that the release manifest carries a SHA-256 digest per uploaded file and is itself served by the download surface.

`distribution/nix-overlay` and `distribution/release-index` name no paths, so the Nix move amends neither. This change archives before `station-replication`, which rewrites the same `station-deployment` requirement; that change re-derives its MODIFIED block from the archived text when its turn comes, and this change only updates the path it cites.

## Non-goals

- Submitting any flavored package manifest to its external home (the Homebrew tap, the AUR, `microsoft/winget-pkgs`, the Chocolatey community repository). The workflow that renders on a successful release, opens the pull requests, and applies per-upstream cadence rules is a follow-up with its own credentials and testing.
- The self-update setting and launch flag the AUR package relies on. `self-update-policy` owns them, with the matching `nix-overlay` amendment.
- Any other package channel: scoop (wants a portable archive Windows does not publish), Flathub and the Snap Store (a sandboxed install cannot self-update and the app has no notify-only mode yet), Debian and Fedora source packaging (need a vendored-source tarball), self-hosted apt/dnf repositories (need a GPG key). None gets a directory, a template, or a registry entry; the design's appendix keeps the notes. The deb and rpm keep being built and published to the store as direct downloads.
- A package for the `outpost` Station CLI. It publishes no release of its own yet; it joins the registry once Waypoints has been test-run end to end.
- Committing flavored package manifests. The renderer and a fixture release manifest validate every channel and release channel in CI; the external repositories are the renders' home and history.
- Any new release artifact. Linux publishes AppImage, deb, and rpm; Windows publishes the NSIS installer; macOS publishes the dmg; every channel here is served from that set.
- Code-signing the Windows installer. winget and Chocolatey can be rendered and validated without it; submitting to either waits on a signing identity, which is its own change.
- Changing what the overlay builds, how the release index is written, or the index's format. Only paths move.
- Changing what the Kubernetes manifests deploy. Open changes that extend them keep their substance; only the paths they cite move.

## Impact

- **Tree**: `deploy/` (8 files) and `nix/` (overlay, channel reading, two package recipes, release index, devenv modules, `AGENTS.md`) removed; `distribution/` created with `channels.toml`, `justfile`, `kubernetes/`, `nix/` (with its own `justfile` for the pins recipes), `brew/`, `aur/`, `winget/`, `chocolatey/`; `tooling/devenv/` created.
- **Task runner**: root `justfile` (`mod deploy` → `mod dist`; `nix-pins` and `nix-pins-check` removed in favor of `dist::nix::pins` and `pins-check`); comments in `deploy/kubernetes/base/kustomization.yaml`, `statefulset.yaml`, and `kind/kind-config.yaml` that name `just deploy::…` recipes.
- **Flake and overlay**: `flake.nix` (five `./nix/…` references: the overlay twice, `channel.nix`, the image recipe, the app recipe); `distribution/nix/overlay.nix` (`tree = ../..`, the recipe path it imports from a fetched tree, `./packages/…`); `distribution/nix/releases/default.nix` and the two package recipes' comments; `tooling/bin/nix-pins` (`PINS_FILE` and the `callPackage` path).
- **devenv**: `devenv.nix` (`imports = [ ./tooling/devenv ]`, the `readFile` of `sync-deny-targets`, comments; the `deploy` lane renamed `dist`); `tooling/bin/xtauri`'s comment pointing at `android.nix`.
- **xtask**: `releases/index.rs` (`INDEX_DIR`, `PRODUCT_PATHSPEC`, tests) and whatever on `main` has since taken over the `nix/releases/` pathspec (re-derived at rebase); `workflows/release_index.rs`'s reason string and the regenerated `.github/workflows/release-index.yaml`; a new `dist` task family (`render`, `check`); `releases/manifest.rs` writes per-file digests from the staged files the publishing job already holds; `channel.rs` gains `Flavor::executable()`; `workflows/pull_request.rs` gains the `dist` job.
- **Website**: `website/shared/utils/releases.ts`'s `ReleaseManifest` type gains the optional `digests` field; a new route `website/server/routes/api/v1/releases/[channel]/manifest.get.ts` streams the stored object.
- **Toolchain**: `.config/treefmt.toml` (`.nuspec` joins the XML formatter; Ruby is already under rufo, YAML and TOML under oxfmt and tombi; PKGBUILD, `.SRCINFO`, and PowerShell have no formatter in the tree and pass through, since treefmt allows unmatched files); `openspec/config.yaml` (`distribution`'s `code:` → `distribution/`, with `package-channels` added to its area list; `toolchain`'s `code:` → `tooling/devenv`); tera added to xtask's dependencies.
- **Specs**: `openspec/specs/distribution/station-deployment/spec.md` and `openspec/specs/distribution/release-delivery/spec.md` amended; `openspec/specs/distribution/package-channels/spec.md` created.
- **Open changes citing the old paths** (at the pinned baseline; re-derived at rebase): `fleet-roster`, `station-replication`, `outpost-observability` (`deploy/…`, `just deploy::…`); `channel-flavors`, `library-minting`, `release-groups-guards` (`nix/channel.nix`, `nix/packages/…`, `nix/overlay.nix`, `nix/releases/`).
- **Depends on**: `self-update-policy` for the `--no-self-update` flag the AUR package passes.
