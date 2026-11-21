# Design

## Context

Verified 2026-11-09 against `3e3bbd09e` and the remote's tags; the rename in D1 was decided 2026-11-10.

**The nightly today.** `releases/version.rs` spells a nightly as `{target}-nightly.{N}+{YYYY.MM.DD}`. `planners/nightly.rs` finds the latest by parsing every `v*-nightly.*` tag and taking the maximum, bumps `N` when `releases::index::product_commits` finds work since it (a `git log` that excludes `nix/releases/`, so index commits never count), and restamps the date. The store build numbers are read back out of the build metadata: Android's `versionCode` is the date without separators, iOS's `CFBundleVersion` the date with them. On 2026-11-08 TestFlight accepted `2026.11.08`, reported the build as `2026.11.8`, and the upload succeeded.

**Rehearsal today.** A nightly dispatch passes `--build-only` (`_jobs.rs`, `BUILD_ONLY_ON_DISPATCH`); `Plan::build_only` then sets `should_build: true` whatever was decided, so every dispatch builds, and the empty `tag` output skips the draft, the publish, and the withdraw. Preview and stable have no planner job: an empty `tag` input is their build-only, and `helpers::tag` and `tag_condition` carry planner-less expressions for them. `guard_dispatch` rejects a tag paired with a platforms subset. `release-plan` and `release-index` already take `--dry-run`, which prints and writes no file; `release-manifest` writes R2 and has no dry mode; `release-index` resolves its commit from the tag, so it cannot render an entry for a tag that does not exist.

**What a publish writes.** The nightly's publish job runs, in order of retractability: download the staged builds, prepare the store listings, upload to the release store (R2), `gh release edit --draft=false`, mark the point of no return, submit to the Play Store, submit to the App Store, write the channel manifest. The draft release with its notes is an earlier job, and withdraw deletes the record and the tag on a shortfall. Every build job already uploads its staged tree as an Actions artifact, which the publish job downloads. For the stores, `supply` offers `validate_only` and `pilot` offers nothing of the kind: its `skip_submission` still uploads the binary.

**Tag discovery.** Planners read `git tag --list`. On 2026-11-09 ten local-only tags, left by withdraws and the history reset, made a local run decide `nightly.7` where CI, holding only GitHub's two tags, decides `nightly.2`. They were deleted that day; nothing stops new ones.

**Who consumes the tag grammar.** The nix recipe (`package.nix`, `app-version`) accepts any suffix led by `.`, `-`, or `+`; `nix/releases/default.nix` fetches the index entry by rev and hash; the website (`shared/utils/releases.ts`) classifies by `-nightly.`, reads the series by prefix, and compares versions by a regex that captures everything after the patch, with fixtures in the old shape; `release-stage` names files after the tag, and the R2 keys carry those names. Two nightlies are published: `v0.1.0-nightly.1+2026.10.25` (objects in R2, no release record) and `v0.1.0-nightly.2+2026.11.08` (objects, the channel manifest, the index entry, and a release record). The `protected-releases` ruleset lets only the wp-botswain App create `v*` tags and has no deletion rule.

**Surroundings.** Every bot write lands through a self-merging pull request (`protected-branches`, archiving before this change). The required checks are job display names written to `tooling/infra/github.auto.tfvars.json`; none is renamed here. The 11-08 scheduled nightly published end to end — tag, index pull request #158 merging itself, Play internal track, TestFlight — and build-only dispatches on 11-07 and, from a branch, 11-08 built every target without tagging.

## Goals / Non-Goals

**Goals:**

- A nightly's version orders by its date under SemVer alone, and the planner's "is there a newer one?" question has one honest answer.
- Every release workflow and the cut can be run end to end with nothing written anywhere but the run's own artifacts, and what a real run would have written is readable from them.
- One gating model: every channel reads the same planner outputs, and a dry run differs from a real run only in where its writes go.
- A local planner run and a CI planner run decide the same thing.

**Non-Goals (design-level, beyond the proposal's):**

- No second nightly on one day under any flag. The date is the version; the stores already refuse a reused build number.
- No second grammar. The two published nightlies are renamed rather than parsed in their old shape.
- No new workflow, job, or required check. The dry counterparts are steps in the jobs that exist.
- No store-side verification of a dry run (D6).
- No change to what a real run writes, or in what order.

## Decisions

### D1 (2026-11-09, decided — John; rename 2026-11-10): The date is the prerelease identifier, and the only grammar

- **Grammar:** `{target}-nightly.{YYYYMMDD}`, no build metadata. One constructor on `Version` takes the target and the UTC date; `first_nightly`, `next_nightly`, and `stamped` go, since they exist to count and restamp. `date_stamp` emits `%Y%m%d`. A nightly parses only in this shape — the prerelease is `nightly` and one 8-digit identifier, and build metadata is refused.
- **Why it orders:** an 8-digit numeric identifier compares numerically, so a later date is a greater version and no tie is left for a crate to break.
- **The earlier grammar is not read.** A tag matching `v*-nightly.*` that does not fit the grammar fails the planner, naming the tag; ignoring it would let a stale tag vote silently. The two published nightlies are renamed instead (Migration Plan): the tags, the release record, the R2 objects, the channel manifest, and the index entry. The Play listing's `versionName` for the 11-08 build stays `0.1.0-nightly.2+2026.11.08`; it is store-side history and names no tag. Rejected: reading both shapes — a second grammar kept forever for two tags, and under the old planner a renamed tag is a counter to bump, so the two cannot coexist in a tag list.
- **Three outcomes:** _New_ when nothing is published for the target, or product work landed since the latest nightly and today's version differs from it; _Current_ when nothing landed, naming the published nightly as it is, no longer restamped — a restamp would be a different version; _Taken_ when work landed but today's version equals the latest tag, which is a second scheduled-style run on a day that already has a nightly. A scheduled run publishes _New_ and skips the other two, each with its own message; _Taken_ says the work ships with the next scheduled nightly. A withdrawn nightly deleted its tag, so a re-run the same day is _New_ again; a run that failed past the point of no return kept its tag and a re-run is _Taken_, which is right, since the stores hold the build.
- **Build numbers** come from the identifier: Android's `versionCode` is the identifier itself, iOS's `CFBundleVersion` is `YYYY.MM.DD` rebuilt from it, zero-padded as today. Apple reports that back as `2026.11.8` and accepts the upload, so the stamp stays as it is; it changes only if Apple starts refusing it.
- **Hotfix policy** (decided): a revert that ships with the next scheduled nightly; halting the rollout in the Play and App Store consoles covers the gap. The stores would refuse a lower build number anyway.
- **Rejected:** the current shape (unordered by spec, reused counter); `nightly.{N}.{YYYYMMDD}` (two identifiers, `N` still reused, and a day never holds two); a same-day counter in build metadata, `nightly.20261108+2` (unordered again, and the stores refuse the second build's `versionCode`, so it could never publish).
- **Gate:** tests — `0.1.0-nightly.20261109 > 0.1.0-nightly.20261108`; `v0.1.0-nightly.2+2026.11.08` fails to parse as a nightly and fails the planner by name; `channels_stay_ordered` under the new grammar; `build_numbers` from the identifier; the three outcomes, including _Taken_ on a fixture repository with a commit after today's tag; a dry run against the renamed tags names `v0.1.0-nightly.{today}`.

### D2 (2026-11-09, decided — John): `--dry-run` and `--[no-]build` replace `--build-only`

| Invocation             | Decides  | Mints tag | Builds | Publish steps |
| ---------------------- | -------- | --------- | ------ | ------------- |
| real run               | yes      | yes       | yes    | for real      |
| `--dry-run`            | yes      | reported  | yes    | dry (D5)      |
| `--dry-run --no-build` | yes      | reported  | no     | none          |
| `--no-build` alone     | rejected |           |        |               |

- **Why two flags:** `--build-only` conflated "do not publish" with "build regardless", and had no answer for a run that should only decide. A dry run decides what the real run decides; `--build` says whether the expensive stages run at all.
- **The decision is reported; `--build` is obeyed.** A dry run reaches the real decision and says so, and when asked to build it builds `{target}-nightly.{today}` whatever that decision was — on a quiet day, and on a day whose nightly already exists. A rehearsal is for exercising the stages, the day's version is the only one it can stamp, and it publishes nothing, so stamping a version the stores already hold is harmless. Rejected: building only when the real run would — a quiet `main` could never be rehearsed, and after the day's nightly nothing could be until the next UTC day.
- **`--no-build` requires `--dry-run`,** refused by the CLI before anything is read, because a real run with no builds would mint a tag with no artifacts behind it.
- **Outputs:** `tag` (minted, or the would-be tag), `tag_minted` (true only when this run created the ref), `dry_run`, `should_build` (the decision on a real run, the `build` flag on a dry one), `version`, `version_code`, `bundle_version` — the last three filled whenever something builds. The withdraw job keeps reading `tag_minted`; the dry counterparts read `dry_run`. `Plan::build_only` goes; `Plan::SKIP` serves a real run's _Current_ and _Taken_.
- **Reporting:** the planner prints its plan and writes the decision, the would-be tag, and the version to the job summary, so a dry run's answer is readable without opening a log.
- **Gate:** CLI tests for each row; `the_prepare_job_republishes_exactly_the_plans_fields` holds the outputs to the job.

### D3 (2026-11-09, proposed): One planner job on every channel

- **Why:** the flags are planner flags, and preview and stable have no planner. Gating them by expressions alone would spell the same decisions in two grammars, and a tag-less dry run needs a would-be tag that only the tree's manifest can supply. Nightly's `prepare` already publishes every output the other jobs read; making it every channel's removes `Option<PrepareJob>` from `build`, `publish`, `withdraw_release`, and `create_release`, and the planner-less arms of `helpers::tag`, `tag_condition`, and `guard`.
- **Shape:** `cargo xtask publish-plan <channel> [--tag <tag>] [--dry-run] [--build|--no-build]`, replacing `nightly-plan`. Nightly refuses `--tag` and mints its own. Preview and stable require `--tag` on a real run (the pushed ref, or the dispatch's existing tag) and on a dry run default it to `v{manifest version}`. The planner refuses a tree whose manifest classifies as another channel, which `src-tauri/build.rs` would refuse later and at greater cost.
- **Outputs are the same seven on every channel.** `version_code` and `bundle_version` stay empty off nightly, as the build jobs already expect; store build numbers for preview and stable mobile builds remain `library-minting`'s follow-up.
- **Cost:** about six minutes before preview and stable builds start — compiling `xtask` through the registry snapshot and sccache, as nightly pays today. The job needs no App token off nightly.
- **Rejected:** expression-only gating (`inputs.dry_run`, `inputs.build`, `inputs.tag`) with the would-be tag read by a shell one-liner in the publish job.
- **Gate:** every channel's workflow has a `prepare` job; `the_manifest_write_depends_on_every_build_job` and `the_withdraw_depends_on_every_build_job` pass with `prepare` in every `needs`; a preview dry run from `main` is refused naming the nightly manifest.

### D4 (2026-11-09, decided — John; shape proposed): Dispatch inputs, and every dispatch defaults to dry

| Workflow          | Inputs                                               | Dry when           |
| ----------------- | ---------------------------------------------------- | ------------------ |
| `release-nightly` | `build` (true), `platforms`                          | always on dispatch |
| `release-preview` | `tag`, `dry_run` (true), `build` (true), `platforms` | `dry_run`          |
| `release-stable`  | the same                                             | `dry_run`          |
| `release-plan`    | `transition`, `dry_run` (true)                       | `dry_run`          |
| `release-index`   | `tag`, `dry_run` (true)                              | `dry_run`          |

- **Nightly** has no `dry_run` input: the schedule is the only way a nightly publishes, so a dispatch is always dry and the planner gets `--dry-run` from `github.event_name == 'workflow_dispatch'`.
- **Default true** everywhere else: a dispatch that forgets the flag rehearses instead of minting or re-publishing, so a real transition, re-publish, or index repair is asked for explicitly. Preview and stable publish for real on a tag push, which has no inputs; a dispatch is a re-publish only when it names a tag and turns `dry_run` off. `release-index`'s dispatch is the repair path for an existing tag; it defaulted to a real write and now defaults to dry like the rest, rendering to a file under D5.
- **Expressions:** `${{ github.event_name == 'workflow_dispatch' && inputs.dry_run && '--dry-run' || '' }}` and `${{ github.event_name == 'workflow_dispatch' && !inputs.build && '--no-build' || '' }}`. The event guard comes first: on a push `inputs.build` is null, and `!null` is true.
- **The guard job** rejects a tag paired with a platforms subset only on a real run; a dry run may build a subset (D5 says what it yields). The no-build-without-dry-run refusal is the CLI's (D2), so a dispatch with `dry_run` off and `build` off fails at the planner. The guard's message drops "leave tag empty for a build-only run".
- **`release-plan`** passes `--dry-run` straight through; its landing steps (`land_write`, `auto_merge`, `wait_for_merge`, `tag_merge_commit`, `bump_main`) gate on `!inputs.dry_run`, and the artifact upload (D5) on `inputs.dry_run`.
- **Rejected:** a three-way `mode` choice input (the CLI has two flags, and a choice would be a third grammar); `dry_run` defaulting to false (a forgotten flag would mint).
- **Gate:** generator tests hold each workflow to its inputs and defaults, the planner step to the two expressions, and the guard's condition to `!inputs.dry_run`.

### D5 (2026-11-09, decided — John; counterparts proposed): A dry run's writes go to the run's artifacts

| Write              | Real run                         | Dry run                   |
| ------------------ | -------------------------------- | ------------------------- |
| Tag                | `create_tag` (nightly)           | reported, `tag_minted` no |
| Release notes      | `gh release create --draft`      | artifact                  |
| Builds             | artifacts, then R2               | artifacts                 |
| Store upload       | `release-upload`                 | skipped                   |
| Release record     | `gh release edit --draft=false`  | skipped                   |
| Point of no return | `submitted=true`                 | skipped                   |
| Store submissions  | `xfastlane publish`              | skipped (D6)              |
| Channel manifest   | `release-manifest`               | artifact                  |
| Index entry        | `release-index.yaml` on tag push | artifact                  |
| Withdraw           | discard, delete, untag           | job skipped               |

- **Why the artifact store:** it is the one place a run can write that nobody reads as a release — not the store, not the forge, not a tag, not a branch — and the builds already go there. A reviewer opens the run and reads what would have been published.
- **Notes:** `git-cliff --unreleased --tag "$TAG"` when the tag does not exist, which is every nightly dry run; `--latest` when it does, which is a preview or stable dry run against an existing tag. The real draft step stays `--latest`.
- **Manifest:** `release-manifest --tag "$TAG" --dry-run` renders the body to a file and builds no store client, so the step resolves no credential.
- **Index entry:** `release-index --tag "$TAG" --rev "$SHA" --dry-run` renders the entry for a tag that does not exist yet, from the commit it would point at — `HEAD` of the planner's checkout on nightly, the tag's commit or `HEAD` on the others. The channel-agreement check runs against that commit's manifest, so a dry run exercises it. `nix flake prefetch` needs the commit on GitHub, which a dispatched ref is.
- **One artifact per run,** `dry-run-<channel>`, from `${RUNNER_TEMP}/dry-run/`: `notes.md`, `manifest.json`, `nix/releases/<channel>.json`, kept for seven days (`retention-days: 7`); a dry run is read the day it runs. The builds stay in `waypoints-<platform>-<channel>` under the default retention.
- **A subset yields builds only.** A dry run with a platforms subset records fewer targets than the channel requires, so the publish job skips on the completeness count as it would for real, and the run's artifacts hold the builds and nothing else. Letting the counterparts run on an incomplete set would rehearse a publish no real run can make.
- **Gates:** the publish job's condition is unchanged (every target recorded, `tag != ''`); each real write step gains `if: dry_run != 'true'` and its counterpart `if: dry_run == 'true'`. The withdraw job gains `&& dry_run != 'true'`: a dry run against an existing tag must never reach `gh release delete`, and it created nothing to withdraw. `mark_submitted` is skipped in a dry run, so `submitted` stays empty.
- **The cut:** `release-plan <transition> --dry-run` writes `series.patch` and `main.patch` (the version writes, computed without leaving a change in the tree), `refs.md` (series branch, tag, both pull requests' heads, titles, and bodies), and `notes.md` (`git-cliff --unreleased --tag`) under the same directory; the workflow uploads `dry-run-release-plan`, also for seven days. No branch, pull request, or tag is created, and the tree is clean afterwards, as `protected-branches` task 3.1 already requires.
- **Rejected:** a dry write to a scratch prefix in R2 (a second place to clean up, under a credential a dry run should not hold); a draft GitHub release that is deleted at the end (a write and a withdraw to rehearse not writing); a dry-run index pull request (the index workflow fires on a tag push, and none happens).
- **Gate:** `the_publish_steps_run_in_order_of_retractability` still holds over the real steps; a new test holds every real write to `dry_run != 'true'` and every counterpart to `== 'true'`; `every_publish_step_resolves_only_its_own_scope` extends to the counterparts resolving nothing; `nightlys_withdraw_deletes_the_tag_and_the_others_do_not` gains the dry-run conjunct; a nightly dry run from a branch leaves `dry-run-nightly` with all three files and nothing in R2 or on the forge.

### D6 (2026-11-09, decided — John): A dry run submits to neither store

- **Why skip:** `pilot` (`upload_to_testflight`) has no validate or verify option, and `skip_submission` still uploads the binary, which consumes a build number Apple will not accept again — the one act the point of no return exists to fence. `supply` (`upload_to_play_store`) has `validate_only`, but a date-based `versionCode` is taken once the day's nightly has published, so a dry run after that fails on a taken number and one before it passes: the result would say whether the nightly has run yet, not whether the lane works. Exercising one store and not the other also rehearses an order of steps no real run has.
- **What is lost:** nothing the 11-08 publish did not already prove. Both lanes ran end to end that day; the dry run's value is in the planner, the version, the gates, and the rendered writes, which the real run exercises only once a day and the others never.
- **Shape:** `submit` steps gain `if: dry_run != 'true'`, nothing else. If fastlane gains a dry mode for TestFlight, the counterpart is one step per store behind the same gate.
- **Rejected:** `validate_only` on Play alone (asymmetric, and fails by whether the day's nightly has run); `skip_submission` on TestFlight (uploads anyway).

### D7 (2026-11-09, proposed): Planners discover tags from the remote

- **Why:** a planner's decision is a claim about what is published, and the remote is where that is. The 11-09 local run voted with tags GitHub no longer had. In CI the checkout fetched its tags from `origin` moments earlier, so reading the remote changes nothing there; locally it makes `--dry-run --no-build` answer what CI would.
- **Shape:** `nightly_tags` and `stable_tags` run `git ls-remote --tags --refs origin 'v*'` and read the names. `commits_since` still names the tag, which the checkout has fetched; a tag the remote has and the checkout lacks fails `git log` loudly, which is the right failure. A planner then needs the network, as `release-index`'s prefetch already does.
- **Rejected:** documenting `git fetch --prune --prune-tags` (advice, not a mechanism); a `--remote` flag (two behaviours to reason about); fetching inside the planner (a write to the checkout from a dry run).
- **Gate:** a local `publish-plan nightly --dry-run --no-build` with a stale local tag decides what CI decides; the fixture tests keep exercising `latest_nightly` and `last_stable` over tag lists, which do not change.

## Risks / Trade-offs

- [The rename window] → between group 1 merging and the rename, a scheduled nightly fails naming an old tag and mints nothing; done the same morning, before the schedule fires, there is no window. The rename is the grammar's point of no return: after it the old planner would read a renamed tag as a counter to bump, so group 1 reverts cleanly only before it.
- [The tag grammar changes under consumers] → every consumer moves with it (D1): the website's parser and fixtures, the index entry, the R2 keys, the comments in `package.nix` and `nix/releases/default.nix`. The nix suffix check accepts a `.`-led extension as it is.
- [A dry run against an existing tag] → the withdraw job is gated off in a dry run (D5), so `gh release delete` never sees a published record; every real write step is gated too, so a missed gate on one step shows up in the generator test, not in R2.
- [`inputs.build` is null on a push] → the event-name guard leads every flag expression (D4), and a generator test holds the rendered expression.
- [Six more minutes on preview and stable] → accepted (D3); mostly cache hits, and a release run takes most of an hour anyway.
- [The planner needs the network] → accepted (D7); an offline planner run fails naming `origin`, rather than deciding on stale tags.
- [A re-run on a taken day publishes nothing] → intended (D1): the stores hold that day's build; the work ships tomorrow. The message says so.
- [A dry build on a taken day stamps the published version] → harmless (D2): it publishes nothing, and the store build numbers it carries are never submitted.
- [`xfastlane prepare` renders `--latest` notes in a dry run] → they describe the previous tag; the step writes nothing outside the runner. Listed as a follow-up.

## Migration Plan

1. **The grammar, the planner outcomes, and the index entry** (D1) in one pull request, with the rename (tasks 1.4) done right after it merges and before that day's scheduled nightly: the two tags re-created under the App's bypass and the old ones deleted, the release record re-pointed with `gh release edit --tag`, the R2 objects moved to their new names, the channel manifest rewritten by `release-manifest`. The next scheduled nightly mints `v0.1.0-nightly.{date}`, above both renamed tags.
2. **The flags and the planner on every channel** (D2, D3, D4), the dry counterparts (D5, D6), and the regenerated workflows, in one landing: a workflow whose inputs changed before its planner did would reject its own dispatch.
3. **The cut's dry run** (D5) alongside or after 2; `release-plan`'s `--dry-run` already prints, so the artifact is additive.
4. **Remote tag discovery** (D7), any time.
5. **First dry runs:** a nightly dry run from `main`, a preview dry run from a scratch series branch, and a dry cut from `main`, each read from its artifacts.

Rollback: steps 2 to 4 revert cleanly, and step 2 reverts to the build-only dispatch by regenerating. Step 1 reverts only before its rename; afterwards the old planner misreads the renamed tags, so a problem there is fixed forward.

## Follow-ups

- `xfastlane metadata` rendering `--unreleased --tag` for the would-be tag in a dry run, so the store listing matches the notes artifact.
- A TestFlight dry mode, if fastlane ever offers one (D6).
- Store build numbers for preview and stable mobile builds, already `library-minting`'s follow-up; the planner outputs they would fill now exist on every channel.

## Open Questions

None.
