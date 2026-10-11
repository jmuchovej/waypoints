## 1. Planner pathspec (independently landable)

- [ ] 1.1 Add `-- . ':(exclude)nix/releases/'` to `nightly_plan.rs::commits_since`'s `git log` argument list; verify a new unit test shows an index-only history yields an empty commit list and `next_nightly` therefore returns `None`
- [ ] 1.2 Add the same pathspec to `release_plan.rs::commits_since`; verify a new unit test shows `cut_target` is unchanged whether or not index commits sit in the history
- [ ] 1.3 Verify both planners' existing test suites still pass and `prek run` is green (clippy, cargo-test, treefmt)

## 2. Index files and their reader

- [ ] 2.1 Commit `nix/releases/{stable,preview,nightly}.json` in the documented empty shape; verify all three parse with `nix eval --expr 'builtins.fromJSON (builtins.readFile ./nix/releases/stable.json)'` (and the other two)
- [ ] 2.2 Write `nix/releases/default.nix` as the only reader — parses a channel's file, derives the version as `lib.removePrefix "v"` of the tag, and raises the D7 eval failure on an absent or empty entry; verify the error message names the channel and points at the index file
- [ ] 2.3 Verify each of the three channels fails evaluation with exactly that message while its entry is empty, and that no channel's failure affects another's

## 3. Hash discovery probe

- [ ] 3.1 Write `nix/releases/probe.nix` taking `rev` and `hash`, fetching that rev and exposing the frontend FOD with `hash = lib.fakeHash`; verify it evaluates without reading any index file
- [ ] 3.2 Run the probe by hand against an existing tag whose lockfile matches `main`'s; verify the scraped `got:` value equals the recorded `sha256-zFz9iUIhZQ66U5lSjCQ+RKiAXfUC7HcWuEAMd2CKgcU=`
- [ ] 3.3 Verify `nix-prefetch-github jmuchovej waypoints --rev <tag> --json` returns a `hash` that, substituted into `fetchFromGitHub`, fetches without a mismatch

## 4. Package rewiring

- [ ] 4.1 Change `nix/packages/waypoints/package.nix` to take its `src` from `fetchFromGitHub { inherit rev hash; }` driven by the channel's index entry, keeping the `root`/`lib.fileset` path available for in-tree builds; verify both paths evaluate
- [ ] 4.2 Thread the version from the index tag into the build via the existing third `--config` layer (the one that store-points `frontendDist`); verify `nix log` shows the tag's version being injected, not the fetched manifest's
- [ ] 4.3 Pass `pnpmHash` from the index entry into `_frontend.nix` in place of the hardcoded hash; verify the frontend derivation builds with the recorded value and fails loudly with a wrong one
- [ ] 4.4 Move the in-tree fileset build to a flake `checks` attribute so it is exercised but is not a consumer-facing package; verify `nix flake show` lists exactly `waypoints`, `waypoints-preview`, `waypoints-nightly` as packages
- [ ] 4.5 Hand-write one real index entry for an existing tag and build that channel end to end; verify the resulting bundle reports the tag's version and carries that channel's product name and bundle identifier

## 5. Release-index workflow

- [ ] 5.1 Add a three-way tag classifier (nightly prerelease → nightly, other prerelease → preview, no prerelease → stable) with unit tests covering all three plus a nightly tag that today's `channel_for` would misfile as preview
- [ ] 5.2 Write `tooling/xtask/src/tasks/workflows/release_index.rs` — tag-push trigger, `steps::install_nix()` + `steps::cachix(…)`, the two hash steps, then the index commit — and register it in `workflows.rs`; verify `just check-generated` is clean and `.github/workflows/release-index.yaml` appears
- [ ] 5.3 Implement the index write reusing `release_plan.rs`'s `commit_file(path, "main", …)`; verify a dry-run mode prints the JSON it would commit without committing
- [ ] 5.4 Run the workflow with its write step disabled against an existing tag; verify both hashes are computed and printed and nothing is committed
- [ ] 5.5 Enable the write and verify the committed file matches the dry-run output byte for byte

## 6. Validation against the specs

- [ ] 6.1 Verify the first real nightly publish fills `nightly.json` and that rebuilding `waypoints-nightly` on a later date reports the published date stamp, not the build date
- [ ] 6.2 Verify pinning a ref of `main` and building `waypoints` produces the published stable release, not a stable-branded build of `main`'s tree
- [ ] 6.3 Verify advancing the pinned ref across a commit that changed only one channel's entry rebuilds that channel and leaves the other two byte-identical
- [ ] 6.4 Verify a simulated failed index write (tag created, write aborted) leaves the channel resolving its previous entry and the workflow re-runnable to completion
- [ ] 6.5 Verify all three channels build in `main`'s CI, so a recipe change that breaks an older tagged source is caught (design.md's first risk)
- [ ] 6.6 Verify co-installability, closure hygiene, and notify-only update behavior are unchanged from `nix-flake-overlay`'s spec now that sources come from tags
