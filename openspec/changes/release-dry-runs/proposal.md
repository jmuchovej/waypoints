# Proposal

## Why

A nightly is versioned `{target}-nightly.{N}+{YYYY.MM.DD}`, and the shape is wrong in three ways the 11-08 nightly made visible. Build metadata has no precedence under SemVer (§10, §11), so two nightlies that differ only in date compare equal, and the planner's choice of the latest nightly works only because the `semver` crate breaks ties on build metadata. `N` is reused: withdrawn nightlies had their tags deleted, so `nightly.2` named every build from 10-27 to 11-06 and the 11-08 one too. And the date cannot simply move into the prerelease as written, because numeric identifiers refuse leading zeros (`2026.11.08` is not a prerelease; `20261108` is). The date is already the store build number, which is why a nightly ships at most once a day; the version should say so.

The only rehearsal the pipeline has is the nightly's `--build-only` dispatch, which builds everything and then skips the publish job, so the release notes, the channel manifest, the index entry, and every publish gate are exercised only by a real publish. Preview, stable, and the cut have no rehearsal at all, and the first real cut is still outstanding. A local planner run reads local tags, and on 11-09 stale local-only tags made it decide `nightly.7` where CI, which holds only GitHub's tags, would decide `nightly.2`.

Why now: this lands before `library-minting`, whose first minting cut is the first real cut, so a dry-run cut is how that cut gets rehearsed. Two nightlies are published under the old shape, and they are renamed rather than parsed: one grammar, and nothing kept for the sake of two tags.

## What Changes

- **A nightly is named by its UTC date.** `{target}-nightly.{YYYYMMDD}`, for example `0.1.0-nightly.20261108`, tag `v0.1.0-nightly.20261108`, with no build metadata. One nightly per UTC day; a run on a day whose nightly already exists publishes nothing and says the work ships with the next scheduled nightly. A hotfix is a revert that ships with the next nightly; halting the rollout in the store consoles covers the gap. Store build numbers stay date-derived (Android `versionCode` `20261108`, iOS `CFBundleVersion` `2026.11.08`). **BREAKING (tag grammar):** the old shape is no longer read — a tag in it fails the planner by name — and the two published nightlies are renamed: tags, release record, R2 objects, channel manifest, index entry.
- **Two planner flags replace `--build-only`, on every channel and the cut.** `--dry-run` decides exactly as a real run would, mints nothing, and reports the would-be tag and version; `--[no-]build` (default on) says whether the test and build stages run, and a dry run asked to build builds the day's version whatever it decided. `--no-build` without `--dry-run` is refused, since it would mint a tag with no artifacts behind it.
- **Dry runs happen only by dispatch, never on a schedule.** A nightly dispatch is always dry; its inputs become `build` and `platforms`. Preview, stable, the cut, and the index repair gain or keep a `dry_run` input defaulting to true, so a real transition, re-publish, or repair is asked for explicitly; preview and stable keep `tag` and `platforms` and gain `build`.
- **Every channel runs the planner.** Nightly's `prepare` job becomes every channel's: it carries the two flags, publishes one set of outputs (`tag`, `tag_minted`, `dry_run`, `should_build`, `version`, and the store build numbers), and every downstream gate reads them. Preview and stable take the tag that triggered the run, or on a dry run the one the tree's manifest would mint.
- **A dry run's writes go to the run's artifact store.** The release notes, the channel manifest, and the index entry are rendered and uploaded as Actions artifacts, kept a week; the builds are already uploaded as artifacts and stay there. Nothing reaches R2, no GitHub release is created or edited, no tag is minted, no pull request is opened, no store is submitted to, and the withdraw job does not run.
- **A dry-run cut reports and renders.** It names the series branch and tag, the two pull requests' branches and titles, and uploads the version-write diffs and the notes as artifacts, leaving the tree clean.
- **Planners read the remote's tags.** Tag discovery runs against `origin`, so a local dry run decides what CI decides.
- The dispatch guard that rejects a tag paired with a platforms subset applies to real runs only; a dry run may build a subset, and then yields builds and nothing else.

## Capabilities

This change stays inside the `distribution` domain.

### New Capabilities

None.

### Modified Capabilities

- `distribution/release-planning`: one requirement is `MODIFIED` — _The app manifest is the sole source of a build's version_ describes the publish-time suffix as "a nightly's counter and date stamp"; it becomes the date-stamped prerelease identifier. Four are `ADDED`: the nightly's date grammar, the one-per-day rule, and the refusal of the old shape; a dry run decides as a real run, mints nothing, and builds when asked, with `--no-build` only beside it; a dry-run cut reports its refs and renders its writes; and planners discover tags from the remote. `protected-branches`'s two `MODIFIED` requirements in this spec are untouched here.
- `distribution/release-publishing`: _Each channel elects what it publishes_ is `MODIFIED` — a run invoked with no tag was build-only; now a dry run elects nothing and a real run with no tag is refused. One requirement is `ADDED`: a dry run's writes go to the run's own artifacts and nothing else, and it withdraws nothing.
- `distribution/release-index`: _Index entry contents_ is `MODIFIED` to drop "build-metadata" from the nightly's date stamp and update the example tag. One requirement is `ADDED`: a dry run renders the entry it would record, under the same channel check, and writes no index file.

`distribution/release-delivery`'s _A downloaded file is saved under a readable name_ stays as published: it permits, and does not require, a date in place of a counter, and the new version carries its date already. `distribution/version-integrity` is untouched.

## Impact

- **`tooling/xtask/src/releases/version.rs`:** `first_nightly`, `next_nightly`, and `stamped` collapse into one constructor taking the date; a nightly parses only in the new shape; `build_numbers` reads the prerelease identifier; the format tests move to the new grammar.
- **`tooling/xtask/src/planners/nightly.rs`:** `date_stamp` emits `YYYYMMDD`; `decide` gains the taken-day outcome and stops restamping a quiet day; an old-shape tag fails the planner by name; `Plan` gains `dry_run` and loses `build_only`; `nightly_tags` reads `origin`.
- **`tooling/xtask/src/bin/xtask/tasks/`:** `nightly_plan.rs` becomes `publish_plan.rs`, taking the channel, `--tag`, `--dry-run`, and `--build`; `release_plan.rs`'s `--dry-run` renders its artifacts; `release_index.rs` gains `--rev` for a would-be tag; `release_manifest.rs` gains `--dry-run`, rendering to a file.
- **`tooling/xtask/src/planners/release.rs`:** `stable_tags` reads `origin`; the dry run computes its diffs without leaving changes in the tree.
- **`tooling/xtask/src/workflows/`:** `_jobs.rs` loses `BUILD_ONLY_ON_DISPATCH` and `Plan::build_only`, `prepare` takes a channel, `create_release`, `publish`, and `withdraw_release` gain their dry counterparts and gates; `_steps.rs` gains the dry render and upload steps and drops the build-only wording from `reject_tag_with_platforms_subset`; `_helpers.rs` loses the planner-less tag expressions; `release.rs` wires `prepare` on every channel and the new inputs; `release_plan.rs` gains `dry_run` and the artifact upload; `release_index.rs`'s `dry_run` defaults to true. `.github/workflows/release-{nightly,preview,stable,plan,index}.yaml` are regenerated. No required check is renamed.
- **`nix/releases/nightly.json`:** the entry's tag becomes `v0.1.0-nightly.20261108`; its rev and hash are unchanged.
- **`nix/packages/waypoints/package.nix`, `nix/releases/default.nix`:** comments naming the old grammar; the suffix check already accepts a `.`-led extension.
- **`website/shared/utils/releases.ts`, `releases.test.ts`:** the version comparison and the fixtures move to the new grammar, with no case for the old shape.
- **GitHub, R2:** the two published nightlies' tags, the one release record, the R2 objects, and the channel manifest are renamed once, by hand, under the App's bypass for the tags.
- **Release pipeline:** the next scheduled nightly mints `v0.1.0-nightly.202611DD`; every dispatch of a release workflow is a rehearsal unless it says otherwise.

## Non-goals

- No dry submission to either store. `pilot` has no validate mode and `skip_submission` still uploads a binary; `supply`'s `validate_only` fails once the day's nightly has taken the `versionCode`. See design D6.
- No change to preview or stable version grammar, to the four transitions, or to how the store build numbers are derived beyond reading them from the new identifier.
- No reading of the old nightly shape anywhere: not the planner, not the website, not the index.
- No rehearsal, from within a dry-run cut, of the preview release that would follow it; `release-preview`'s own dry run covers that.
- No store-listing notes for the would-be tag: `xfastlane prepare` keeps rendering `git-cliff --latest`, writes nothing outside the runner, and in a dry run may describe the previous tag.
- No new release workflow, job display name, or required check.
