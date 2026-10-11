## Context

See proposal.md for motivation. Current state that shapes the approach — each verified against the tree:

- `nix/overlay.nix` is `{root ? ../., rustToolchain ? null}: final: _prev:` calling `packages/waypoints/package.nix` once per flavor. Source selection is entirely `root`-anchored `lib.fileset` (prior change's D4), so today every flavor is the pinned ref's tree.
- `nightly_plan.rs::commits_since` is `git log --format=%s --reverse <range>` with **no pathspec**; `next_nightly` returns `None` when `VersionUpdater` leaves the prerelease unchanged, which is the entire quiet-repo skip. Any commit in range defeats it.
- `release_plan.rs::commits_since` is the same unfiltered shape, and feeds `cut_target` (`VersionUpdater` with `with_features_always_increment_minor`). So index commits perturb the `cut` target as well — the proposal's scope covers both planners, not just nightly.
- `release_plan.rs` already writes cross-branch to `main`: `write_version("main", next, …)` on a cut, via `commit_file`, which uses the GitHub Contents API (`PUT repos/{repo}/contents/{path}` with a `branch` field). It needs no committer identity, no push credentials, and no deep checkout. A `// TODO: if main ever becomes a protected branch, botswain will only be able to commit through a PR` already sits there. **So "release CI needs write access to `main`" is not a new cost — it is today's situation, and the mechanism to reuse already exists.**
- `nightly_plan.rs::create_tag` tags `git rev-parse HEAD` at plan time, through the same refs API. `release_plan.rs` tags the version-bump commit it just created and therefore already knows that rev.
- Workflows are _generated_ from `tooling/xtask/src/tasks/workflows/*.rs` (gh-workflow) and registered in `workflows.rs`; `.github/workflows/*.yaml` are outputs guarded by `just check-generated`. `steps::install_nix()` and `steps::cachix(…)` already exist and are used by `check_generated` and `devenv_update`.
- The pnpm FOD hash of record is `sha256-zFz9iUIhZQ66U5lSjCQ+RKiAXfUC7HcWuEAMd2CKgcU=` (prior change's D5), and it is a hash of the _dependency set_, not of the source tree.

## Goals / Non-Goals

**Goals:**

- A published release is reconstructible from the repository alone, at any later date, without consulting the GitHub Releases API or any other live service beyond fetching the tag.
- The three-attribute overlay surface and every other requirement of `distribution/nix-overlay` survive unchanged.
- Release tooling gains exactly one behavioral change (the pathspec) and one new workflow; the planners' decision logic is otherwise untouched.
- Hash discovery is mechanical and scripted, never a maintainer's manual step.

**Non-Goals (design-level, beyond the proposal's):**

- No schema versioning or migration machinery for the index files. Three files, one shape, one writer, one reader; if the shape changes, all four change together in one commit.
- No attempt to make the index human-editable as a supported workflow. It is machine-written; hand-editing is a repair operation, not an interface.
- No verification that a recorded rev is reachable from the branch its channel is supposed to live on. Tags are the contract; branch topology is not re-checked.

## Decisions

### D1 (2026-09-27): One JSON file per channel, flat, latest-only

`nix/releases/{stable,preview,nightly}.json`, each a flat object:

```json
{
  "tag": "v0.2.0-nightly.7+2026.08.29",
  "rev": "9f3c1a…",
  "hash": "sha256-…",
  "pnpmHash": "sha256-…"
}
```

- **One file per channel, not one combined file**: two channels can publish minutes apart, and each write is a blind `PUT` against a blob sha through the Contents API. Separate files mean separate blobs and therefore no conflict window at all, rather than a retry loop. It also makes the `git log` exclusion (D5) a single directory instead of a set of paths.
- **Latest-only, not a history**: a history array reintroduces the conflict the split removed (both channels appending to the same list, and even one channel appending across a force-retry), and nothing in the spec wants an older release buildable. Older releases remain in git history if anyone wants them.
- **JSON, not TOML or Nix**: `builtins.fromJSON` has no TOML-1.0/1.1 hazard (prior change's D10 exists precisely because `builtins.fromTOML` is strict 1.0), and the writer serializes it without a TOML library.
- **No `version` field**: it would be the tag minus its `v`, and two fields naming one value can disagree. `lib.removePrefix "v" tag` is the derivation.
- **Rejected**: recording the version instead of the tag — the tag is also what the fetch needs, so the tag is strictly more informative.

### D2 (2026-09-27): The build recipe comes from `main`; the source comes from the tag

`main`'s `nix/packages/waypoints/*` is the recipe for all three channels; the index only supplies which source it is applied to. `src` becomes the `fetchFromGitHub` output rather than a `lib.fileset`, and the fileset narrowing disappears for index builds (its two jobs — escaping git filtering and keeping copies small — are both moot for a codeload tarball of a tagged tree).

- **Why not evaluate the tagged tree's own Nix code**: because a tag's recipe is _stale_, not absent. Checked against the repo (2026-09-27): no `v0.*.x` branches exist yet, and the single existing tag — `v0.1.0-nightly.1+2026.09.23`, an ancestor of `main` — does carry `nix/`, including `nix/packages/waypoints-cli/package.nix` and `nix/packages/waypoints/flavors/*.nix`, files since deleted, plus a `package.nix` predating the root-plumbing repair, the Linux appindicator dependency, the flavor-channel stamp, and manifest-sourced versions. Evaluating that tag's own build logic would faithfully reproduce a build that does not work. Since release branches are cut from `main`, future tags will carry `nix/` too — always as of when they were cut. Fetching a tag's _source_ is well-defined; importing its _build logic_ pins you to whatever was broken at the time.
- **Consequence, accepted**: `main`'s recipe must keep building older tagged sources. A recipe change that assumes a file only present on newer branches silently breaks the stable package. Mitigated by building all three attributes in CI on `main` (see Migration Plan), which is the only thing that would catch it.
- The `root`-anchored fileset path is **kept**, not deleted — it is how a working copy is built locally and how `nix flake check` on `main` exercises the recipe. It is simply not what `waypoints`/`waypoints-preview`/`waypoints-nightly` use.
- **Rejected**: a fourth `waypoints-local` package attribute for the in-tree build. The spec forbids presenting an in-tree build as a channel, and an unbranded fourth attribute in the consumer-facing overlay invites exactly the confusion this change removes. The in-tree path stays a flake `checks`/internal attribute.

### D3 (2026-09-27): Hash discovery is a two-step probe, both steps driven from `main`

The two hashes are found differently, and the difference matters:

1. **Source hash** — `nix-prefetch-github jmuchovej waypoints --rev <tag> --json`, read `.hash`. Pure fetch, no evaluation, no chicken-and-egg.
2. **pnpm hash** — must be _built_, and building it needs a source, which is the chicken-and-egg the index would otherwise create. Resolved by a standalone probe expression on `main` — `nix/releases/probe.nix`, taking `rev` and `hash` as arguments and exposing the frontend FOD with `hash = lib.fakeHash` — so the workflow runs `nix build -f nix/releases/probe.nix …` against the tag _before_ any index entry exists, and scrapes the `got: sha256-…` line from the failure. Only then is the file written.

- **Why the probe rather than parameterizing the real package**: the real package reads the index by construction; giving it an index-bypassing argument would add a second source-selection channel to the thing whose whole point is having one.
- **Useful property**: the pnpm hash is a hash of the resolved dependency set, not of the source tree, so it is _identical_ across releases whose `pnpm-lock.yaml`, `pnpm-workspace.yaml`, and `patches/` agree. The probe will usually confirm the previous value rather than discover a new one. It still must be recorded — `fetchPnpmDeps` needs it at eval time while `fetchFromGitHub` yields a store path at build time — but the failure mode of getting it wrong is loud and immediate, not subtle.
- **Rejected**: computing the pnpm hash from the checked-out tag using the tag's own Nix code — same objection as D2 (tags on release branches have none).
- **Rejected**: `--impure` + `builtins.readFile` of the fetched store path at eval time to sidestep recording it. Impure eval in a consumer's overlay is not acceptable, and it would defeat the flake's own reproducibility.

### D4 (2026-09-27): A dedicated tag-push workflow writes the index, not the planners

A new generated workflow (`tooling/xtask/src/tasks/workflows/release_index.rs`, registered in `workflows.rs`, output `.github/workflows/release-index.yaml`) triggers on tag push, classifies the tag into a channel from its version (prerelease containing `nightly` → nightly; any other prerelease → preview; none → stable), runs D3's two steps, and commits the channel's file to `main` via the existing `commit_file` path.

- **Why a separate workflow rather than a step in the planners**: the write must happen strictly after the tag exists (spec: _the index is written after its tag exists_), and a tag-push trigger encodes that ordering in the platform rather than in our control flow. It also keeps the Nix toolchain — `install_nix`, cachix, a real FOD build — out of the planner jobs, which today need only `gh` and a shallow checkout.
- **Why not reuse `release_plan.rs`'s `channel_for`**: it classifies _any_ prerelease as preview, so it would file nightlies under preview. The index writer needs a three-way classifier. Whether that lives as a third variant of `channel_for` or a new function is an implementation call; the three-way behavior is not.
- The write reuses `commit_file(path, "main", …)` verbatim — no new credential model. The protected-branch `TODO` already in `release_plan.rs` now applies to one more caller; noted, not solved here.
- **Rejected**: writing the index from the release workflow's `publish` job — it would couple index freshness to bundle upload success, and a nightly's `build-only` dispatch path has no tag at all.

### D5 (2026-09-27): The pathspec exclusion goes in both planners

`commits_since` in `nightly_plan.rs` **and** `release_plan.rs` appends `--`, `.`, `:(exclude)nix/releases/` to the `git log` argument list.

- **Why both**: nightly's is the loud failure (the skip dies, a nightly publishes every day forever). Release's is the quiet one (`cut_target` reads the same history, so an index commit can nudge the next minor). The proposal's framing named only nightly; the tree says both.
- **Why a pathspec rather than filtering by commit message**: the message is ours to choose, but "was this commit only bookkeeping?" is a question about paths, and a path answer cannot be defeated by a future message-format change. It is also what the spec's scenarios describe.
- **Why `-- . ':(exclude)…'` rather than bare `':(exclude)…'`**: a pathspec list consisting only of exclusions matches nothing in git; the positive `.` is required.
- **Gate**: a unit test per planner asserting that a history of index-only commits yields an empty commit list, and that a mixed history yields only the product commits. Both are testable without CI because they are pure over the `git log` output — the pathspec itself needs a small git fixture or an argument-shape assertion, whichever the existing test style supports.

### D6 (2026-09-27): Version from the tag supersedes D9 for overlay builds only

The prior change's D9 made `src-tauri/Cargo.toml`'s `[package] version` the single source of truth. That stays true _for what a branch builds_ — release tooling, the bundler pipeline, and the in-tree Nix path all keep reading it. For the three overlay packages the authority moves to the index's tag, because a nightly's version was never in any manifest to begin with.

- The two cannot disagree in practice: the tag was minted from the manifest (stable/preview) or from the manifest's target plus a date (nightly), and the fetched source at that tag carries the manifest the tag was minted from.
- They _can_ disagree for nightly, and deliberately: the tag carries `-nightly.N+YYYY.MM.DD` while the fetched manifest says the bare target. The tag wins; a third `--config` layer supplies the version, the same mechanism that already store-points `frontendDist`.
- D9's stated consequence — the frontend derivation takes `version` as a parameter from the app derivation, one parse, one source — is preserved. The parse is simply replaced by `lib.removePrefix "v" index.tag`.

### D7 (2026-09-27): Bootstrap is an explicit eval failure, not a fallback

A channel whose file is absent (or whose entry is empty) fails evaluation with a message naming the channel and pointing at the index. No fallback to the pinned tree, to another channel, or to a placeholder version.

- **Why**: every fallback reintroduces the misrepresentation this change removes. A consumer who gets an error learns the truth ("nightly has not published under the index yet"); a consumer who gets a build learns a falsehood.
- The three files are therefore committed as part of this change with a documented empty shape, so the failure is a deliberate, testable state rather than a missing-file crash.
- **Ordering in practice**: nightly fills within a day of landing; preview and stable fill on their next respective publishes, which may be a while. That asymmetry is expected and is why per-channel failure beats all-or-nothing.

### D8 (2026-09-27): Layout

```
nix/
  releases/
    stable.json                # written by release-index CI
    preview.json
    nightly.json
    probe.nix                  # D3 step 2: FOD hash discovery, index-free
    default.nix                # reads a channel's file; the eval-failure boundary (D7)
  packages/waypoints/
    package.nix                # src from fetchFromGitHub (index) or fileset (in-tree)
    _frontend.nix              # takes version + pnpmHash as parameters
tooling/xtask/src/tasks/
  workflows/release_index.rs   # generated workflow (D4)
  nightly_plan.rs              # pathspec (D5)
  release_plan.rs              # pathspec (D5) + three-way channel classification
```

`nix/releases/default.nix` is the only reader of the JSON, so the schema has exactly one consumer and D7's error message has exactly one home.

## Risks / Trade-offs

- **[`main`'s recipe drifts from older tagged sources]** → The stable package can break without anything on `main` changing that looks related. Mitigated by building all three attributes in `main`'s CI, which is the only signal; accepted as the cost of D2's recipe/source split.
- **[The pnpm hash is recorded, and a wrong one blocks the channel]** → Loud and immediate (hash mismatch at build), and the probe that produced it is the same code path as the build that consumes it. The real risk is the probe silently succeeding against a _stale_ dependency set; mitigated by the probe running against the fetched tag, never against `main`'s tree.
- **[Cross-branch write from a tag-push workflow]** → Uses the mechanism `release_plan.rs` already uses to bump `main` on a cut, so the permission surface is unchanged. If `main` becomes protected, this and the existing `write_version("main", …)` both need a PR path — one problem, already flagged in-tree.
- **[A failed index write leaves a published release the overlay cannot see]** → By design (spec scenario): the release is valid, the channel keeps its previous entry, and the workflow is re-runnable because the tag is already there and both hashes are functions of it. Nothing needs undoing.
- **[Consumers see a channel move one commit late]** → Accepted. The alternative is the index describing a tag it cannot hash, which is impossible.
- **[Bootstrap window where `waypoints` and `waypoints-preview` do not evaluate]** → Accepted per D7; the error text is the mitigation. Duration depends on release cadence, which is not this change's to set.
- **[`nix-prefetch-github` and the codeload tarball's stability]** → GitHub's generated archives have changed compression details historically; a recorded `hash` that stops matching would break an old release's rebuild. Noted, not mitigated here; if it bites, the fix is fetching the git tree rather than the tarball, which changes the hash format but nothing else in the design.

## Migration Plan

1. **Pathspec first** (D5), in both planners, with tests. Independently landable and useful on its own — it is a correctness fix to the planners regardless of whether the index ever ships.
2. **Commit the three index files empty** plus `nix/releases/default.nix` (D7, D8), and prove the eval failure is the documented one for all three channels.
3. **Probe expression** (D3 step 2), validated by hand against an existing tag: it must reproduce `sha256-zFz9iUIhZQ66U5lSjCQ+RKiAXfUC7HcWuEAMd2CKgcU=` for a tag whose lockfile matches `main`'s.
4. **Package rewiring** (D2, D6): `src` from `fetchFromGitHub`, version from the tag, `pnpmHash` threaded to the frontend FOD, in-tree fileset path retained behind `checks`. Validated by hand-writing one index entry for a real tag and building it.
5. **The workflow** (D4): generated, `just check-generated` clean, exercised first with its write step disabled (hashes computed and printed, nothing committed), then enabled.
6. **First real publish** on each channel fills its file; verify the built bundle's reported version equals the tag, and for nightly that the date stamp is the published one and not the build date.

Rollback: steps 2–5 are additive to the tree and revert by restoring the `root`/fileset `src` in `package.nix` and deleting `nix/releases/` plus the generated workflow. Step 1 stands on its own and should not be reverted. Consumers who pinned a ref from the index era keep working — their pin still evaluates the code they pinned.

## Open Questions

- Whether the three-way tag classifier lives as a third variant of `release_plan.rs`'s `channel_for` or as a separate function in the index writer. Affects no spec and no task boundary.
- Whether `nix/releases/default.nix` should also expose the parsed entries as a flake output for inspection (`nix eval .#releases`). Convenience only; safely added later.
- Whether the index workflow should skip its own run when the computed entry is byte-identical to the committed one (a re-run, or a retag). Harmless either way — the Contents API `PUT` of identical content is a no-op commit — but the no-op commit would itself be an index commit, which D5 already renders inert.
