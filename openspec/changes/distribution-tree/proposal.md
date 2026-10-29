# Proposal

## Why

Waypoints reaches users today through two channels — the direct download the website serves from the release store, and the Nix overlay — and the tree keeps what it has of distribution in two places that describe neither: the Station's Kubernetes manifests under `deploy/`, and the overlay under `nix/` beside the dev shell's modules, which have nothing to do with shipping. Package-manager channels (Homebrew, the AUR, winget, Chocolatey) each want a manifest that names the same release the store already publishes, and without one home and one derivation they will be hand-copied per channel and per flavor and drift — a first-pass sketch at Homebrew already carried three near-identical casks. This change gives distribution one tree, one registry of which distribution channels each release channel reaches, and one contract for how a channel's manifest derives from the published release.

Tree changes below are stated against commit `ce4067aca`.

## What Changes

- **BREAKING** (task-runner surface): `deploy/` is removed and its contents move into a new top-level `distribution/` tree. The root `justfile`'s `mod deploy "deploy/justfile"` becomes `mod dist "distribution/justfile"`, so `just deploy::<recipe>` becomes `just dist::kubernetes::<recipe>`. The mise tool lane `deploy` (kubeconform, kubectl, kind) is renamed `dist`.
- The Kubernetes tree moves whole: `deploy/kubernetes/{base,kind}/*` → `distribution/kubernetes/{base,kind}/*`, `deploy/justfile` → `distribution/kubernetes/justfile`. Recipe names are unchanged (`lint`, `image`, `kind-up`, `kind-load`, `kind-apply`, `kind-down`).
- **BREAKING** (flake consumers, of which there are none yet): the Nix channel moves under the same tree — `nix/overlay.nix`, `nix/channel.nix`, `nix/packages/`, and `nix/releases/` → `distribution/nix/`. The overlay imports a recorded release's recipe from the tree at that release's revision, so `waypoints.nightly` on `main` fails to evaluate from the merge until the next nightly's index pull request merges a post-move revision; `stable` and `preview` already throw (their index files are `{}`), and nothing in CI evaluates the recorded packages. Accepted: no consumer exists, and the gap closes with the next nightly.
- `nix/devenv/` (the dev shell's modules and the scripts devenv runs as tasks) moves to `tooling/devenv/`, beside the rest of how the repository builds itself; `nix/` is removed. `nix/AGENTS.md` moves with it and is trimmed to the devenv rules.
- `distribution/` adopts one contract: one directory per distribution channel, named for it; a channel's inner layout follows its own tooling's idiom (kustomize's base and overlays, Nix's recipes and release index) and never spills into a second top-level directory; a channel with validation recipes has a `justfile` at its root, and `distribution/justfile` declares a `mod` per such channel. Package channels hold manifest **templates** — never a rendered, version-stamped manifest.
- `distribution/channels.toml` is the registry: one table per release channel (`stable`, `preview`, `nightly`), each entry naming a distribution channel and the package identity that release channel takes there. It is both the renderer's source of identities and the record of which distribution channels serve which release channel.
- Four new channel directories: `brew/` (one Homebrew cask template, installing the dmg on macOS and the AppImage — with its desktop entry — on Linux), `aur/` (`-bin` PKGBUILD and `.SRCINFO`, from the deb), `winget/` (the three-manifest set), `chocolatey/` (nuspec and install script). Every release channel is registered on all four.
- `cargo xtask dist render <channel> --flavor <flavor>` renders a distribution channel's templates — file names and contents — from the release record: the identity from the registry, the product name and bundle identifier from `src-tauri/res/configs/flavors/<flavor>.toml`, the version and series from the channel manifest, download URLs from the release-delivery layout, and artifact digests from the manifest. Rendered output goes to `target/dist/`, never into the tree.
- The channel manifest the publishing job writes (`{channel}.json`) gains a SHA-256 digest per uploaded file, and the download surface serves the manifest itself at `/api/v1/releases/{channel}/manifest`, so a package manifest's checksum derives from the record without a store credential or a re-download.
- The design's appendix keeps the non-normative channel notes — reach, cost, review model, and why five further channels are deferred — for reference.

## Capabilities

### New Capabilities

- `distribution/package-channels`: the in-tree manifests for third-party package channels — where they live, that they are templates deriving every release-specific value from the published record, the registry of which distribution channels each release channel reaches and as what, and how a rendered manifest is validated before it leaves the tree.

### Modified Capabilities

- `distribution/station-deployment`: the requirement "Reference manifests describe a one-Station Fleet that survives pod replacement" names `deploy/kubernetes/` as the manifests' home; it becomes `distribution/kubernetes/`, base and kind overlay nested as before. The requirement "Manifests validate against the Kubernetes API schema" is unchanged in substance; its task-runner recipe is `just dist::kubernetes::lint`.
- `distribution/release-delivery`: the requirement "A channel's manifest is what an updater reads" gains that the manifest carries a SHA-256 digest per uploaded file and is itself served by the download surface, so a downstream tool reads checksums from the record alone.

`distribution/nix-overlay` and `distribution/release-index` name no paths, so the Nix move amends neither. The open change `station-replication` rewrites the same `station-deployment` requirement wholesale; its delta is updated to the new path so that whichever change archives second carries the other's text.

## Non-goals

- Submitting any rendered manifest to its external home (the Homebrew tap, the AUR, `microsoft/winget-pkgs`, the Chocolatey community repository) — the workflow that renders on a successful release, opens the pull requests, and applies per-upstream cadence rules is a follow-up with its own credentials and testing. This change makes every channel renderable and validated from one record.
- Any other package channel: scoop, Flathub, the Snap Store, Debian and Fedora source packaging, self-hosted apt/dnf repositories. None gets a directory, a template, or a registry entry; the design's appendix records why each waits. The deb and rpm keep being built and published to the store as direct downloads.
- Committing rendered manifests. The renderer and a fixture manifest validate every channel and release channel in CI; the external repositories are the renders' home and history.
- Any new release artifact. Linux publishes AppImage, deb, and rpm; Windows publishes the NSIS installer; macOS publishes the dmg; every channel here is served from that set.
- Code-signing the Windows installer. winget and Chocolatey can be rendered and validated without it; submitting to either waits on a signing identity, which is its own change.
- Changing what the overlay builds, how the release index is written, or the index's format. Only paths move.
- Changing what the Kubernetes manifests deploy. Open changes that extend them (`fleet-roster`, `station-replication`, `outpost-observability`) keep their substance; only the paths they cite move.

## Impact

- **Tree**: `deploy/` (8 files) and `nix/` (overlay, channel reading, two package recipes, release index, devenv modules, `AGENTS.md`) removed; `distribution/` created with `channels.toml`, `justfile`, `kubernetes/`, `nix/`, `brew/`, `aur/`, `winget/`, `chocolatey/`; `tooling/devenv/` created.
- **Task runner**: root `justfile` (`mod deploy` → `mod dist`); comments in `deploy/kubernetes/base/kustomization.yaml`, `statefulset.yaml`, and `kind/kind-config.yaml` that name `just deploy::…` recipes. The root `nix-pins` recipes stay where they are.
- **Flake and overlay**: `flake.nix` (five `./nix/…` references: the overlay twice, `channel.nix`, the image recipe, the app recipe); `distribution/nix/overlay.nix` (`tree = ../..`, the recipe path it imports from a fetched tree, `./packages/…`); `distribution/nix/releases/default.nix` and the two package recipes' comments; `tooling/bin/nix-pins` (`PINS_FILE` and the `callPackage` path).
- **devenv**: `devenv.nix` (`imports = [ ./tooling/devenv ]`, the `readFile` of `sync-deny-targets`, comments; the `deploy` lane renamed `dist`); `tooling/bin/xtauri`'s comment pointing at `android.nix`.
- **xtask**: `releases/index.rs` (`INDEX_DIR`, `PRODUCT_PATHSPEC`, tests); `workflows/release_index.rs`'s reason string and the regenerated `.github/workflows/release-index.yaml`; a new `dist` task family (`render`, `check`); `releases/manifest.rs` writes per-file digests, computed from the staged files the publishing job already holds.
- **Website**: `website/shared/utils/releases.ts`'s `ReleaseManifest` type gains the optional `digests` field; a new route `website/server/routes/api/v1/releases/[channel]/manifest.get.ts` serves the manifest as published.
- **Toolchain**: `.config/treefmt.toml` (Ruby already formats through rufo/rubocop; `.nuspec` joins the XML formatter; PKGBUILD, `.SRCINFO`, winget YAML, and PowerShell are excluded with a comment, since no formatter for them is in the tree); `openspec/config.yaml` (`distribution`'s `code:` → `distribution/`, with `package-channels` added to its area list; `toolchain`'s `code:` → `tooling/devenv`); tera added to xtask's dependencies.
- **Specs**: `openspec/specs/distribution/station-deployment/spec.md` and `openspec/specs/distribution/release-delivery/spec.md` amended; `openspec/specs/distribution/package-channels/spec.md` created.
- **Open changes citing the old paths**: `fleet-roster`, `station-replication`, `outpost-observability` (`deploy/…`, `just deploy::…`); `channel-flavors`, `library-minting`, `release-groups-guards` (`nix/channel.nix`, `nix/packages/…`, `nix/overlay.nix`, `nix/releases/`, including the latter's `paths = [ "nix/releases/" ]` ruleset).
- **Working tree**: the uncommitted `distribution/` sketches and `nix/justfile` carry a layout this change rejects; they are restored to `ce4067aca` before the move.
