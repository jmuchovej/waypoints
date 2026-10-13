## 1. Planner pathspec (independently landable)

- [x] 1.1 Route `nightly_plan.rs::commits_since` through one shared `git log` helper that appends `-- . ':(exclude)nix/releases/'`; verify against a temp git repository that an index-only history yields an empty commit list and `next_nightly` therefore returns `None`
- [x] 1.2 Route `release_plan.rs::commits_since` through the same helper; verify `cut_target` is unchanged whether or not index commits sit in the history
- [x] 1.3 Verify both planners' existing test suites still pass with clippy, nextest, and treefmt green

## 2. Index files and their reader

- [x] 2.1 Commit `nix/releases/{stable,preview,nightly}.json` as `{}`; verify all three parse
- [x] 2.2 Write `nix/releases/default.nix` as the only reader — requires `tag`, `rev`, and `hash`, derives the version as the tag minus its `v`, and raises the D7 eval failure on an absent or empty entry; verify the error message names the channel and points at the index file
- [x] 2.3 Verify each of the three channels fails evaluation with that message while its entry is empty, and that no channel's failure affects another's

## 3. The version names the channel

- [ ] 3.1 Move `main`'s app manifest to `0.1.0-nightly`; verify the workspace still resolves its version and the nightly planner still derives the bare target from it
- [ ] 3.2 Make `release_plan.rs`'s post-cut bump of `main` carry the `-nightly` suffix; verify with unit tests that a cut plans `X.(Y+1).0-nightly` for `main` and that advance, promote, and patch are unchanged
- [ ] 3.3 Add the three-way classifier (nightly prerelease → nightly, other prerelease → preview, none → stable) and a cross-check that refuses a tag whose version and tagged manifest classify differently; verify with unit tests covering all three channels and a mismatch

## 4. Overlay and package

- [ ] 4.1 Keep `nix/packages/waypoints/package.nix` on the `workingTree` recipe and give it an optional `version` override that must extend the manifest's version; verify an in-tree evaluation keeps the manifest version and a nightly override is injected through the third `--config` layer
- [ ] 4.2 Rewrite `nix/overlay.nix` to read the tree's channel from its manifest version (one shared reading in `nix/channel.nix`) and expose `waypoints = recurseIntoAttrs { stable; preview; nightly; }`: index mode fetches a channel's recorded tag with `fetchTree` and evaluates that tree's `package.nix`; self-build mode applies the overlay's own recipe to its own tree for the version's channel; the other attributes throw naming the ref's channel and series and where to pin; verify by evaluating a tree under each version shape
- [ ] 4.3 Update `flake.nix` to `packages.{stable,preview,nightly,default}` with `default` following the version's channel, keep the development-flavor in-tree build as `checks.waypoints-in-tree`, and update `nix-pins` to call the recipe directly; verify the flake lists exactly those outputs
- [ ] 4.4 Hand-write an index entry for an existing tag and evaluate that channel through the overlay; verify the recipe evaluated is the fetched tree's own (a tag predating the `workingTree` interface fails inside its own `package.nix`, not in the overlay), and that evaluation never copies a working tree into the store

## 5. Release-index workflow and writer

- [ ] 5.1 Write `cargo xtask release-index --tag` — classify, cross-check the manifest at the tag, `nix flake prefetch` the source hash, render the three-field entry, and commit it to `main` via `commit_file`; verify the rendered bytes match what treefmt leaves and that `--dry-run` prints the entry without committing
- [ ] 5.2 Write `tooling/xtask/src/tasks/workflows/release_index.rs` — `v*` tag push plus a dispatch with `tag` and `dry_run`, the devenv-shell job shape (install nix with an access token, cachix, `devenv shell -- cargo xtask release-index`) — and register it; verify `cargo xtask workflows --check` is clean, `.github/workflows/release-index.yaml` appears, and zizmor reports nothing
- [ ] 5.3 Mint refs in the nightly `prepare` job and the `release-plan` job with the `wp-botswain` App token scoped to `contents: write`, dropping their `GITHUB_TOKEN` grant to read; verify the workflow tests' environment/credential pairing holds and the no-cache rule applies only to the release environment
- [ ] 5.4 Run the workflow by dispatch with `dry_run` against an existing tag; verify the hash is computed and printed and nothing is committed
- [ ] 5.5 Run it for real and verify the committed file matches the dry-run output byte for byte

## 6. Validation against the specs

- [ ] 6.1 Verify the first real nightly publish fills `nightly.json` and that rebuilding `nightly` from `main` on a later date reports the published date stamp, not the build date
- [ ] 6.2 Verify pinning a ref of `main` and building `stable` produces the published stable release, not a stable-branded build of `main`'s tree
- [ ] 6.3 Verify advancing the pinned ref across a commit that changed only one channel's entry rebuilds that channel and leaves the other two byte-identical
- [ ] 6.4 Verify a simulated failed index write (tag created, write aborted) leaves the channel resolving its previous entry and the workflow re-runnable to completion from the dispatch
- [ ] 6.5 Cut the first series and verify its branch ref builds `preview` as itself, fails `stable` and `nightly` with the documented messages, and after promotion builds `stable` as itself and fails `preview`
- [ ] 6.6 Verify co-installability, closure hygiene, and notify-only update behavior are unchanged from `nix-flake-overlay`'s spec now that sources come from tags
