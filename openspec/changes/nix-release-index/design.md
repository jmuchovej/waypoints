## Context

See proposal.md for motivation. Current state that shapes the approach — each verified against the tree, and revised on 2026-10-13 after studying Zed's release branching (`vN.M.x` branches, cherry-pick-only series, a channel every tree can read off itself) and the first implementation round:

- `nix/overlay.nix` is `{root ? ../., rustToolchain ? null}: final: _prev:` calling `packages/waypoints/package.nix` once per flavor. Source selection is `root`-anchored `lib.fileset`, so every flavor is the pinned ref's tree. `package.nix` reads the pnpm dependency hash and the git-sourced cargo crate hashes from `nix/packages/waypoints/hashes.json`, refreshed by `tooling/bin/nix-pins`.
- `nightly_plan.rs::commits_since` and `release_plan.rs::commits_since` are `git log --format=%s --reverse <range>` with **no pathspec**. Any commit in range defeats the nightly skip and nudges a cut's target.
- `release_plan.rs` already writes cross-branch to `main` on a cut through the Contents API (`commit_file`), which needs no committer identity and no deep checkout. Both planners mint refs with `GITHUB_TOKEN`, and a ref created that way raises no `push` event, so no tag-triggered workflow has ever fired for a planner-minted tag.
- Workflows are generated from `tooling/xtask/src/tasks/workflows/*.rs`; `steps::install_nix`, `steps::cachix`, `steps::app_token`, and the automation-profile export already exist.
- The repository is private. The github `fetchTree` honours the `access-tokens` a consumer already needs to pin the flake; `fetchFromGitHub` would need credentials inside the sandbox.
- `pkgs.root` is a nixpkgs attribute, so `callPackage` fills in any `root` argument a caller omits. The in-tree argument is named `workingTree` for that reason.

## Goals / Non-Goals

**Goals:**

- A published release is reconstructible from the repository alone, at any later date: the latest of each channel from `main`, any release from its tag, the current release of a series from its branch.
- The commitment "this tree builds its release" is held by the branch that ships it, for as long as that branch is live — never by `main` on behalf of every release ever made.
- The index never grows. Three files on `main`, one entry each.
- The overlay's behaviour on any ref is decided by the manifest version alone, with no fallback and no guessing.
- Hash discovery is mechanical and scripted, never a maintainer's manual step — and as small as possible.

**Non-Goals (design-level, beyond the proposal's):**

- No schema versioning or migration machinery for the index files. Three files, one shape, one writer, one reader; if the shape changes, all five change together in one commit.
- No attempt to make the index human-editable as a supported workflow. It is machine-written; hand-editing is a repair operation, not an interface.
- No verification that a recorded rev is reachable from the branch its channel is supposed to live on. Tags are the contract; branch topology is not re-checked.

## Decisions

### D1 (2026-09-27, revised 2026-10-13): One JSON file per channel, flat, latest-only, three fields

`nix/releases/{stable,preview,nightly}.json`, each a flat object:

```json
{
  "tag": "v0.2.0-nightly.7+2026.08.29",
  "rev": "9f3c1a…",
  "hash": "sha256-…"
}
```

- **One file per channel, not one combined file**: two channels can publish minutes apart, and each write is a blind `PUT` against a blob sha through the Contents API. Separate files mean separate blobs and therefore no conflict window at all.
- **Latest-only, not a history**: an append-only index grows without bound and duplicates what git tags and the release bucket already are. "Any release" is answered by tags and series branches (D11), not by this file.
- **No dependency hashes** (revised): the tag's own tree carries `hashes.json` for its lockfiles, and D2 builds that tree with its own recipe. The only hash nix cannot read from the tree is the tree's.
- **JSON, not TOML or Nix**: `builtins.fromJSON` has no TOML-1.0/1.1 hazard, and the writer serializes it without a TOML library.
- **No `version` field**: it would be the tag minus its `v`, and two fields naming one value can disagree. `lib.removePrefix "v" tag` is the derivation.

### D2 (2026-09-27, reversed 2026-10-13): A tag's tree builds itself, with its own recipe

A channel package on `main` fetches the tree the index entry names and evaluates _that tree's_ `nix/packages/waypoints/package.nix` against it. `main`'s recipe is used only for `main`'s own in-tree build.

- **Why reversed**: the first design applied `main`'s recipe to every recorded source, accepting that the recipe must keep building older trees. That is bounded only while the index is latest-only and the channels are few; it was already the design's first-listed risk. Under D13, a series branch is cut from a `main` whose recipe builds, and the recipe, lockfiles, and pins travel with every cherry-pick. So a tag's recipe is correct for its tree by construction — the objection that tags carried stale recipes described a tree that predates this change.
- **Consequence, accepted**: `package.nix`'s argument list — `flavor`, `workingTree`, `rustToolchain`, `rev`, `version`, `updaterEndpoints` — is an interface a newer overlay calls on an older tag. It is stable from the first real release on; adding an argument with a default is compatible, renaming one is not.
- **Consequence, accepted**: a tag builds against the consumer's nixpkgs, not the tag's `flake.lock`. The same is true of every overlay package, and `hashes.json`'s fixed-output hashes are stable across nixpkgs revisions in practice (`fetcherVersion` is pinned).
- **Rejected**: a history of entries, each pinning the `main` commit its recipe came from. Self-contained, but reintroduces the growth D1 removes, for a guarantee D13 already gives.

### D3 (2026-09-27, revised 2026-10-13): Hash discovery is one prefetch

The writer learns the source hash with `nix flake prefetch --json github:<repo>/<rev>` and reads `.hash`. That is the NAR hash `fetchTree` verifies, obtained through the same fetcher with the same credential, and nothing is built.

- **No probe** (revised): the first design probed the pnpm and cargo fixed-output hashes by building them against a placeholder. With D2 those come from the tag's `hashes.json`, so the probe, its expression, and the mismatch scraping go.
- **Cross-check**: the writer reads the app manifest at the tag and refuses a tag whose version classifies as one channel while the manifest's classifies as another. A nightly tagged on a preview branch, or a stable tag on `main`, was minted wrongly, and the index must not record it.

### D4 (2026-09-27, revised 2026-10-13): A dedicated tag-push workflow writes the index; refs are minted so the push is observable

A generated workflow (`tooling/xtask/src/tasks/workflows/release_index.rs`, output `.github/workflows/release-index.yaml`) triggers on any `v*` tag push, and by dispatch with a `tag` input for a tag that already exists. It runs `cargo xtask release-index --tag <tag>` inside the devenv shell (the shape `check-generated` and `devenv-update` use), which classifies the tag, cross-checks the tagged manifest, prefetches, and commits the channel's file to `main` via `commit_file(path, "main", …)`.

- **Why a separate workflow**: the write must happen strictly after the tag exists, and a tag-push trigger encodes that ordering in the platform rather than in our control flow.
- **App token** (revised): both planners mint their refs with the `wp-botswain` App token, because a ref created with `GITHUB_TOKEN` raises no `push` event. The nightly `prepare` job and `release-plan` job resolve the automation profile, mint a token scoped to `contents: write`, and hand it to the planner; their own `GITHUB_TOKEN` grant drops to read. The version-bump commit a cut makes on `main` is authored by the App too, so `main`'s checks run on it.
- **Caches**: declaring an environment forgoes GitHub's cache actions (mise's own install cache excepted) — the rule `build-cache` fixed for every environment-declaring job, and the workflow tests encode. The planners and the index job compile through the CI cache instead: the build jobs' cargo-registry snapshot and sccache store, under the same keys, so a planner compile is mostly hits. The index job reaches `sccache`, `rclone`, and `xcache` inside the devenv shell, which is why those steps accept which toolchain they run under.
- **Rejected**: writing the index from the release workflow's `publish` job — it would couple index freshness to bundle upload success, and a nightly's `build-only` dispatch has no tag at all.

### D5 (2026-09-27): The pathspec exclusion goes in both planners

`commits_since` in `nightly_plan.rs` **and** `release_plan.rs` reads `git log` through one helper that appends `-- . ':(exclude)nix/releases/'`.

- **Why both**: nightly's is the loud failure (the skip dies, a nightly publishes every day forever). Release's is the quiet one (`cut_target` reads the same history, so an index commit can nudge the next minor).
- **Why a pathspec rather than a message filter**: whether a commit is bookkeeping is a question about paths, and a path answer cannot be defeated by a future message-format change.
- **Why `-- . ':(exclude)…'` rather than bare `':(exclude)…'`**: a pathspec of only exclusions matches nothing in git; the positive `.` is required.
- **Gate**: proven against a real git repository in a temp dir, not by asserting argument strings.

### D6 (2026-09-27, revised 2026-10-13): The tag's version overrides the manifest only where the manifest cannot know it

`package.nix` takes an optional `version`. Index mode passes the tag-derived version; self-build mode passes nothing and the manifest rules.

- For stable and preview tags the two are equal by construction (the tag was minted from the manifest), so the override changes nothing.
- For nightly the tag carries `-nightly.N+YYYY.MM.DD` and the manifest `-nightly` alone; the override is the only way the bundle can report the counter and the published date. It reaches Tauri through the third `--config` layer, the one that already store-points `frontendDist`.
- **Invariant, asserted at evaluation**: the override must extend the manifest's version (`removePrefix manifest override` is empty or starts with `.`, `-`, or `+`). A recorded tag that disagrees with the manifest it points at is a wrong entry, and fails loudly.

### D7 (2026-09-27): Bootstrap is an explicit eval failure, not a fallback

A channel whose file holds `{}` fails evaluation with a message naming the channel and pointing at the index. No fallback to the pinned tree, to another channel, or to a placeholder version — every fallback reintroduces the misrepresentation this change removes. The three files are committed empty with this change, so the failure is a deliberate, testable state.

### D8 (2026-09-27, revised 2026-10-13): Layout

```
src-tauri/
  Cargo.toml                   # version names the channel: -nightly | -pre.N | bare (D9)
nix/
  channel.nix                  # the one reading of a version as a channel (D9)
  releases/
    stable.json                # written by release-index CI (D1, D4)
    preview.json
    nightly.json
    default.nix                # the only reader; the eval-failure boundary (D7)
  overlay.nix                  # reads the channel; index mode or self-build (D10, D11)
  packages/waypoints/
    package.nix                # the recipe, applied to workingTree; `version` override (D6)
    _frontend.nix
    hashes.json
tooling/xtask/src/tasks/
  release_index.rs             # the writer and the planners' shared `git log` (D3, D5)
  nightly_plan.rs              # pathspec (D5)
  release_plan.rs              # pathspec (D5), main's -nightly target (D9)
  workflows/release_index.rs   # generated workflow (D4)
```

### D9 (2026-10-13): The manifest version names the channel on every branch

`main` declares `X.Y.Z-nightly`; a series branch declares `X.Y.Z-pre.N` from its cut and a bare `X.Y.Z` from its promotion. A `nightly` prerelease is the nightly line, any other prerelease a preview, none stable — the same reading `release_plan.rs::channel_of` applies to tags, now applied to trees. The planner already writes every one of these transitions; the only addition is that its post-cut bump of `main` writes `X.Y.0-nightly` rather than `X.Y.0`.

- **Why the version and not a marker file**: the overlay must tell `main` from a stable series branch, and with a bare version on both nothing in the tree could. A separate one-token file (Zed's `RELEASE_CHANNEL`) was the first answer; it worked but duplicated what the version already says on every branch but `main`, and the planners are about to be refactored, so a second channel-bearing file is one more thing for that refactor to carry. The suffix makes the manifest the single source of both version and channel, which `distribution/release-planning` already states for the version.
- **Semver keeps the staircase**: `0.3.0-nightly < 0.3.0-nightly.1+date < 0.3.0-pre.1 < 0.3.0`, so `main`'s manifest sorts below every nightly it will publish and below the series it will cut.
- **Consequence, accepted**: desktop nightly bundles, which take their version from the crate, report `X.Y.Z-nightly` instead of the bare `X.Y.Z` that the eventual stable also reports. That is a distinction they were missing.
- **Consequence, accepted**: "`main` and the preview branch declare the same version" becomes "the same base version", with the base being `major.minor.patch`; the nightly planner already strips the suffix to find its target.
- **Rejected**: `[package.metadata]` in `Cargo.toml`, or a marker file — both say what the version can say.

### D10 (2026-10-13): The package surface is `waypoints.{stable,preview,nightly}`

The overlay exposes `waypoints` as a `recurseIntoAttrs` set of three derivations; the flake exposes `packages.{stable,preview,nightly}` with `default` following the version's channel (D11). `pkgs.waypoints.nightly` in a module and `github:jmuchovej/waypoints#nightly` on the command line are the two spellings.

- **Why an attribute set**: the channel name is the attribute on every ref, and on a series ref the attribute that is not the tree's channel fails independently of the one that is.
- **Rejected**: `waypoints` as the stable derivation with the other two hanging off it. Evaluating `pkgs.waypoints.nightly` would then force stable's entry to exist, and an unpublished stable would break nightly.
- **Rejected**: keeping `waypoints`, `waypoints-preview`, `waypoints-nightly` alongside — one surface, not two.

### D11 (2026-10-13): A series ref is its own release; `main` is the latest-tracking target

| ref                      | `preview`               | `stable`             | `nightly`   | `default` |
| ------------------------ | ----------------------- | -------------------- | ----------- | --------- |
| `main` (`X.Y.Z-nightly`) | index entry             | index entry          | index entry | `stable`  |
| series, `X.Y.Z-pre.N`    | the tree                | fails: no stable yet | fails       | `preview` |
| series, bare `X.Y.Z`     | fails: no more previews | the tree             | fails       | `stable`  |
| a tag                    | as its manifest says    |                      |             |           |

- In index mode (D2) the fetched tree's recipe is evaluated with `version` from the tag and `rev` from the entry.
- In self-build mode the overlay's own recipe is applied to the overlay's own tree, with `rev` from the flake when available.
- A failing attribute names the ref's channel and series (from the manifest's major.minor) and says where to pin: `main` for the latest of any channel, the next series for previews after a promotion.
- **Rejected**: a per-branch index so `preview` keeps resolving after promotion. A branch's pointers are always one tag behind on the tag's own tree, so tag pins would still need the version-named path — two mechanisms on one ref, to hand a preview user code that shipped as stable. Preview users pin `main#preview` or the newest series.
- **Nightly by version**: a nightly tag's tree is `main`, whose manifest says `-nightly`, so pinning it hits index mode and resolves the previous nightly. A specific past nightly is the `main` commit that recorded it, which the index commit's message names; the release bucket is the archive. Accepted, and the same answer Zed gives.

### D12 (2026-10-13): Sources are fetched with the github `fetchTree`

Index mode fetches with `fetchTree { type = "github"; … narHash = entry.hash; }` at evaluation time. The repository is private, and this fetcher reads the `access-tokens` a consumer already needs to pin the flake; `fetchFromGitHub` would need credentials inside the sandbox. The hash is the same NAR hash either way, so switching fetchers later needs no index rewrite. Fetching at evaluation is also what lets the fetched tree's `package.nix` be imported without import-from-derivation; the store-path string is turned back into a path value by discarding its (empty) context.

### D13 (2026-10-13): Series branches carry only cherry-picks of `main`

A series branch is cut from `main`'s head, then receives (a) the planner's version commits and (b) cherry-picks of commits already on `main`, in the order they landed there, through pull requests into the branch. Nothing lands on a series branch first. Exactly two series are live at a time — the current preview and the current stable — and a branch stops receiving changes when the next series is promoted.

- **Why this is a design decision and not just process**: D2 rests on it. A cherry-pick carries the recipe, lockfile, and pin changes that accompany a fix, so a tag's tree is always one `main` knew how to build. A series branch that took changes `main` never had would break that.
- **Why PRs rather than direct pushes**: the branch's history is the release notes' source, and `(cherry-pick to preview|stable)` in a subject is the audit trail.
- Mechanism (a `cherry-pick` workflow, as Zed has) is out of scope here; the policy is what the spec states.

## Risks / Trade-offs

- **[`package.nix`'s interface is called across versions]** → A newer overlay evaluates an older tag's recipe. Keep the argument list additive; a rename breaks every recorded tag at once and shows up as an eval error naming the attribute. The in-tree check and the three index builds on `main` are the signal.
- **[A tag builds against the consumer's nixpkgs]** → A fixed-output hash in a tag's `hashes.json` could stop matching under a far newer fetcher. Loud and immediate; the fix is a cherry-picked pin refresh while the series is live, and acceptance that an end-of-life series builds against nixpkgs of its era.
- **[Cross-branch write from a tag-push workflow]** → Uses the mechanism `release_plan.rs` already uses to bump `main` on a cut. If `main` becomes protected, this and the existing write both need a PR path — one problem, already flagged in-tree.
- **[A failed index write leaves a published release the overlay cannot see]** → By design: the release is valid, the channel keeps its previous entry, and the workflow is re-runnable from the dispatch because the tag is already there.
- **[A tag minted on the wrong branch]** → Its version and the tagged manifest's version classify differently; the writer refuses it, so the mistake cannot reach the index.
- **[Something reads `main`'s manifest version as a bare number]** → The planners strip the suffix where they need the target; the bundler passes it through, which is the intended change. Anything else is a consumer of the manifest to find, and the test suite's manifest-reading tests run on every branch.
- **[Bootstrap window where `main`'s `stable` and `preview` do not evaluate]** → Accepted per D7; series refs evaluate from day one.
- **[A preview pin breaks on promotion]** → Deliberate (D11): the error says where previews continue. Preview users pin `main#preview`.

## Migration Plan

1. **Pathspec first** (D5), in both planners, with tests. Independently landable.
2. **Index files empty** plus the reader (D1, D7); prove the eval failure for all three channels.
3. **Channel in the version** (D9): `main`'s manifest moves to `0.1.0-nightly`; the cut's `main` bump keeps the suffix; the classifier reads it.
4. **Overlay and package** (D2, D6, D10, D11, D12): the attribute set, index mode evaluating a fetched tag's recipe, self-build on a version match, the failing attributes' messages, the flake's packages and in-tree check. Validated by evaluating index mode against a hand-written entry for an existing tag, and by evaluating a tree under each version shape.
5. **The workflow and writer** (D3, D4): generated, `just check-generated` clean, dry-run against an existing tag, then the App-token minting in both planners.
6. **First real publish** on each channel fills its file; verify the built bundle's reported version equals the tag, and for nightly that the date stamp is the published one and not the build date. Cut the first series and verify its ref builds itself as preview, then as stable.

Rollback: steps 2–5 are additive and revert by restoring the previous overlay. Step 1 stands on its own. A series branch cut under this design keeps building itself whatever `main` does afterwards.

## Open Questions

- Whether `nix/releases/default.nix` should also expose the parsed entries as a flake output for inspection (`nix eval .#releases`). Convenience only; safely added later.
- Whether the index workflow should skip its own run when the computed entry is byte-identical to the committed one. Harmless either way — the no-op commit would itself be an index commit, which D5 renders inert.
- Whether to publish the three `main` closures to cachix from the index workflow, which is the moment a channel moves. A separate change; it does not touch what the index records.
