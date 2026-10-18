## Why

The Nix overlay landed by `nix-flake-overlay` builds `waypoints`, `waypoints-preview`, and `waypoints-nightly` from whichever ref the consumer pins. But a flake ref pins one commit, on one branch, carrying one version — so of the nine flavor×ref combinations only three are meaningful. The other six produce artifacts that misrepresent their channel: `nix build github:jmuchovej/waypoints#waypoints` against `main` yields a stable-branded, `so.waypoints`-identified bundle of nightly code, which also collides on disk and in LaunchServices with a real stable install.

Nightly is the sharpest case, and it is not merely mislabeled but _unrecoverable_. `nightly_plan` computes `{target}-nightly.{N}+{YYYY.MM.DD}` and emits it as a CI plan, creating the tag through the GitHub refs API without ever committing the version anywhere. Flake metadata exposes `self.rev` but never the tag name that points at it. So no tree, at any ref, on any date, can tell a Nix build what a published nightly's version was — and the date stamp, being wall-clock at plan time, cannot be re-derived by a build running later.

A Nix user asking for "the nightly" wants the nightly that was published, not a same-named rebuild of whatever `main` looks like today. That requires the repo to record what was published. A Nix user asking for `0.2.3` wants exactly that release, which requires the tree tagged `v0.2.3` to build itself as a stable release — and to keep doing so.

## What Changes

- Add a **committed release index**: one file per channel, `nix/releases/{stable,preview,nightly}.json`, living on `main`. Each records the most recently published release of that channel — its tag, the rev the tag points at, and the source hash that pins the fetched tree. Nothing else: the tree itself carries its dependency hashes.
- Make the **manifest version name the channel** on every branch: `main` declares `X.Y.Z-nightly`, a series branch (`v{major}.{minor}.x`) declares `X.Y.Z-pre.N` until promoted and a bare `X.Y.Z` after. A tree thereby knows which channel it ships as from the one file that already versions it, and nothing else needs writing. The release planner's post-cut bump of `main` keeps the suffix.
- **BREAKING (for overlay consumers):** the package surface becomes `waypoints.stable`, `waypoints.preview`, and `waypoints.nightly` (flake outputs `stable`, `preview`, `nightly`, and `default`). On `main`, each fetches the tree its channel's index entry names and evaluates that tree's own recipe. On a series branch or tag, the attribute matching the tree's version builds the tree itself; the other two fail to evaluate with a message naming the ref and where to pin instead. Pinning `main` is a latest-tracking target; pinning a series or a tag is a narrowed or exact one.
- Add a release-index CI job that runs **after** a tag exists, prefetches the tag's source hash, and commits the channel's index file to `main`.
- Mint every release ref with the `wp-botswain` App token rather than `GITHUB_TOKEN`, so a tag's push is observable by the workflows that depend on it.
- Fix `commits_since` in the release planners to exclude the index directory (`-- . ':(exclude)nix/releases/'`). Without this, an index commit on `main` is itself "work that landed", which manufactures the justification for the next nightly and destroys the quiet-repo skip.
- State the series-branch policy: a series branch receives only cherry-picks of commits already on `main`, in the order they landed there, so a tag always points at a tree whose recipe, lockfiles, and pins were cut from a known-good `main` and maintained together.

## Capabilities

`nix-flake-overlay` introduced the **`distribution`** domain (packaging, installation, and update delivery outside the Tauri bundler pipeline). This change stays inside it. It also needs a home for release-_planning_ invariants — tag minting, the quiet-repo skip, what counts as landed work, what a branch declares about itself — which no registered domain covers and which `nix-flake-overlay` did not spec. Rather than mint a second domain for one capability, it lands as `distribution/release-planning`: the planners exist to decide what gets published, which is the front half of distribution.

### New Capabilities

- `distribution/release-index`: the committed per-channel record of published releases — what each entry holds, which invariants bind it (one entry per channel, the recorded hash pins the tag's source exactly, the index is metadata and never code), who writes it and when relative to the tag, and how a missing entry behaves.
- `distribution/release-planning`: invariants the release and nightly planners must hold for the index and the channel packages to be safe — a nightly is published only when product work landed since the last one, commits that only touch the release index are not product work, every branch's manifest version names its channel, series branches carry only cherry-picks of `main`, and release refs are minted so their pushes are observable.

### Modified Capabilities

- `distribution/nix-overlay`: the package surface and the source each package builds. The three flat attributes become `waypoints.{stable,preview,nightly}`; on `main` each builds the tree its index entry names with that tree's own recipe, and on a series ref the attribute matching the tree's version builds the tree itself. The existing requirements on flavor mapping, co-installability, wrapper layout, frontend embedding, and notify-only updates are untouched.

  Note: `openspec/specs/` does not exist yet — `distribution/nix-overlay` lives only as `nix-flake-overlay`'s unsynced delta. This change's delta is therefore written against that pending spec and must be sequenced after it, or synced together with it.

## Impact

- **`nix/releases/{stable,preview,nightly}.json`** (new, committed on `main`) and **`nix/releases/default.nix`** (their only reader): the index. One file per channel so concurrent releases never textually conflict and the `git log` exclusion is a single directory. Three files, forever: history lives in tags, series branches, and the release bucket, not here.
- **`src-tauri/Cargo.toml`** on `main`: `version` gains the `-nightly` prerelease. Desktop nightly bundles, which take their version from the crate, report `X.Y.Z-nightly` rather than a bare `X.Y.Z` that the eventual stable would also report.
- **`nix/channel.nix`** (new): the one reading of a manifest version as a channel, shared by the overlay and the flake. **`nix/overlay.nix`**: index mode fetches a tag and evaluates its `nix/packages/waypoints/package.nix`, self-build mode calls the overlay's own. **`nix/packages/waypoints/package.nix`** keeps the `workingTree` recipe it had and gains a version override, which is the one thing a nightly's tag knows and its manifest does not. Its argument list becomes an interface a newer overlay calls on an older tag, and is kept stable from the first real release on.
- **`flake.nix`**: `packages.{stable,preview,nightly,default}`; the in-tree development build stays a flake check.
- **`tooling/xtask/src/tasks/nightly_plan.rs`** and **`release_plan.rs`**: `commits_since` gains the exclusion pathspec in both; `release_plan`'s post-cut `main` bump carries the `-nightly` suffix.
- **`tooling/xtask/src/tasks/release_index.rs`** (new) and **`workflows/release_index.rs`** (new): the writer and its tag-push workflow. The writer classifies the channel from the tag, cross-checks it against the tagged tree's manifest version, prefetches the source hash, and commits through the Contents API path `release_plan.rs` already uses. The nightly `prepare` job and `release-plan` job mint refs with the App token.
- **Bootstrap gap**: the index cannot describe releases that predate it, so on `main` each channel's package is unevaluable until that channel publishes once under the new pipeline. Series refs are unaffected — they build themselves from day one.
- **Consumer-visible latency**: on `main`, a channel's package moves one commit after each publish rather than continuously.

## Non-goals

- A history inside the index. Each channel's file records the latest release only; every release remains reachable as a tag or series ref, and as artifacts in the release bucket.
- A per-branch index. A series branch's own tree is its release; carrying pointers there would always be one tag behind on the tag's own tree.
- Retrieving a specific past nightly through the overlay by version. A nightly's tree does not know its date; the `main` commit that recorded it does, and the release bucket keeps the artifact.
- Windows, Android, or iOS artifacts, and the dmg/nsis/appimage bundler pipeline — unchanged and still owned by the release workflows.
- Binary cache (cachix) publishing of the resulting packages for consumers.
- Making the index the source of truth for anything other than Nix builds. The app manifest on each branch remains authoritative for what that branch builds, per `nix-flake-overlay`'s D9.
- Changing the branch/version model itself (`main` nightly, `v0.2.x` preview, `v0.1.x` stable; `v0.1.0` / `v0.2.0-pre.N` / `v0.2.0-nightly.N+YYYY.MM.DD`) — the manifest suffix names it, it does not redefine it.
