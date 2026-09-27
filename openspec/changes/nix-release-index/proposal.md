## Why

The Nix overlay landed by `nix-flake-overlay` builds `waypoints`, `waypoints-preview`, and `waypoints-nightly` from whichever ref the consumer pins. But a flake ref pins one commit, on one branch, carrying one version — so of the nine flavor×ref combinations only three are meaningful. The other six produce artifacts that misrepresent their channel: `nix build github:jmuchovej/waypoints#waypoints` against `main` yields a stable-branded, `so.waypoints`-identified bundle of nightly code, which also collides on disk and in LaunchServices with a real stable install.

Nightly is the sharpest case, and it is not merely mislabeled but _unrecoverable_. `nightly_plan` computes `{target}-nightly.{N}+{YYYY.MM.DD}` and emits it as a CI plan, creating the tag through the GitHub refs API without ever committing the version anywhere. Flake metadata exposes `self.rev` but never the tag name that points at it. So no tree, at any ref, on any date, can tell a Nix build what a published nightly's version was — and the date stamp, being wall-clock at plan time, cannot be re-derived by a build running later.

A Nix user asking for "the nightly" wants the nightly that was published, not a same-named rebuild of whatever `main` looks like today. That requires the repo to record what was published.

## What Changes

- Add a **committed release index**: one file per channel, `nix/releases/{stable,preview,nightly}.json`, living on `main`. Each records the most recently published release of that channel — its tag, the rev the tag points at, the `fetchFromGitHub` source hash, and the pnpm fixed-output-derivation hash.
- **BREAKING (for overlay consumers):** each of the three packages stops building the pinned ref's working tree and instead `fetchFromGitHub`s the rev its channel's index entry names, deriving its version from the recorded tag. Pinning a newer ref of the Waypoints flake then means "get a newer _index_", not "rebuild my channel from this commit". The three-attribute surface and the co-installability guarantee are unchanged.
- Add a release-index CI job that runs **after** a tag exists, computes both hashes against the tag, and commits the channel's index file to `main`.
- Fix `commits_since` in the release planners to exclude the index directory (`-- . ':(exclude)nix/releases/'`). Without this, an index commit on `main` is itself "work that landed", which manufactures the justification for the next nightly and destroys the quiet-repo skip. This is a behavioral invariant, not an implementation detail, which is why it needs a spec home.
- Document the bootstrap gap: until a channel has published a release under the new pipeline, that channel's index has no entry and its package does not evaluate.

## Capabilities

`nix-flake-overlay` introduced the **`distribution`** domain (packaging, installation, and update delivery outside the Tauri bundler pipeline). This change stays inside it. It does need a home for release-_planning_ invariants — tag minting, the quiet-repo skip, what counts as landed work — which no registered domain covers and which `nix-flake-overlay` did not spec. Rather than mint a second domain for one capability, it lands as `distribution/release-planning`: the planners exist to decide what gets published, which is the front half of distribution.

### New Capabilities

- `distribution/release-index`: the committed per-channel record of published releases — what each entry holds, which invariants bind it (one entry per channel, the recorded hashes pin the tag's source exactly, the index is metadata and never code), who writes it and when relative to the tag, and how a missing entry behaves.
- `distribution/release-planning`: invariants the release and nightly planners must hold for the index to be safe — a nightly is published only when product work landed since the last one, and commits that only touch the release index are not product work.

### Modified Capabilities

- `distribution/nix-overlay`: the version-and-identity source for each package. Currently each package's version comes from the app manifest of the pinned ref; it changes to come from its channel's index entry, and the package's source changes from the pinned tree to the fetched tag. The existing requirements on flavor mapping, co-installability, wrapper layout, frontend embedding, and notify-only updates are untouched.

  Note: `openspec/specs/` does not exist yet — `distribution/nix-overlay` lives only as `nix-flake-overlay`'s unsynced delta. This change's delta is therefore written against that pending spec and must be sequenced after it, or synced together with it.

## Impact

- **`nix/releases/{stable,preview,nightly}.json`** (new, committed on `main`): the index. One file per channel rather than one combined file, so concurrent releases on different channels never textually conflict and the `git log` exclusion pathspec is a single directory.
- **`nix/packages/waypoints/package.nix`**, **`nix/packages/waypoints/_frontend.nix`**, **`nix/overlay.nix`**: source shifts from the `root`-anchored `lib.fileset` to `fetchFromGitHub` driven by the index; version comes from the recorded tag rather than a parsed manifest (superseding the one-parse-one-source consequence of `nix-flake-overlay`'s D9 for overlay builds). The `root`-based path stays for in-tree `nix build`/`nix flake check`.
- **`tooling/xtask/src/tasks/nightly_plan.rs`** and **`release_plan.rs`**: `commits_since` gains the exclusion pathspec in both. `release_plan`'s `cut` transition feeds the same unfiltered `git log` into `cut_target`, so index commits perturb the next minor target there too — not just nightly's skip.
- **`tooling/xtask/src/tasks/workflows/`**: a new generated workflow (plus its registration in `workflows.rs`) that fires on tag push, installs Nix via the existing `steps::install_nix`/`steps::cachix` helpers, computes the two hashes, and commits the index. `release_plan.rs`'s `commit_file` already writes to a named branch through the Contents API and already writes to `main` cross-branch on a cut, so the mechanism and the permission it needs both exist today.
- **Hash discovery is the fiddliest part** and it is a cost, not a detail: `fetchFromGitHub` yields a store path at build time while `fetchPnpmDeps` needs its hash at _eval_ time. The pnpm hash therefore cannot be derived from the fetched source and must be recorded — CI has to run the `lib.fakeHash` → read the `got:` value dance against the tag and write the answer down.
- **Ordering is forced**: the index records the tag's rev and source hash, so it cannot be written before the tag exists without containing its own hash. The index necessarily lands after the tag, on a commit the tag does not include. For nightly that means the index commit sits on `main` alongside the tagged history, which is exactly why the pathspec exclusion is load-bearing. For stable and preview there is no self-reference at all — the tag is on a release branch, the index is on `main`.
- **Bootstrap gap**: the index cannot describe releases that predate it, so each channel's package is unevaluable until that channel publishes once under the new pipeline.
- **Consumer-visible latency**: a channel's package only moves when the index does, which is one commit after each publish rather than continuous with `main`.

## Non-goals

- Retaining a ref-follows-channel mode, or any package attribute that builds the pinned ref's tree as a released channel. In-tree `nix build` against a working copy stays available and stays explicitly unbranded as a channel.
- Historical entries. Each channel's index records the latest release only; an archive of every published version is a separate concern with separate ergonomics (and would reintroduce the merge conflicts the one-file-per-channel split avoids).
- Backfilling releases published before the index exists.
- Windows, Android, or iOS artifacts, and the dmg/nsis/appimage bundler pipeline — unchanged and still owned by the release workflows.
- Binary cache (cachix) publishing of the resulting packages for consumers.
- Making the index the source of truth for anything other than Nix builds. The app manifest on each branch remains authoritative for what that branch builds, per `nix-flake-overlay`'s D9.
- Changing the branch/version model itself (`main` nightly, `v0.2.x` preview, `v0.1.x` stable; `v0.1.0` / `v0.2.0-pre.N` / `v0.2.0-nightly.N+YYYY.MM.DD`) — the index records it, it does not redefine it.
