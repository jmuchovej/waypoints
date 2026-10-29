# Proposal

## Why

`main` and the `v{major}.{minor}.x` series branches are to become protected branches, landing every change as a merge commit through a pull request gated by required status checks (decided). Nothing protects them today: the one ruleset left on the repository, `default`, is disabled, `GET /rules/branches/main` returns no rules, and the two writes the release planners make to `main` — the cut's provisional version bump in `planners/release.rs` and the release-index entry in `tasks/release_index.rs`, both through the Contents API — each carry a comment deferring the protected-branch case to the change that protects the branch. This is that change. It also lands the lockstep `cartograph` version with the write path, because a lockstep check that lands before the planner writes both manifests fails every pull request after the next transition.

Why now: `release-groups-guards` gives the repository a `version-integrity` check that runs on every pull request. A ruleset can require that check only once it exists and reports, and the planners' direct writes fail the moment a branch ruleset is active — so the rulesets, the pull-request write path, and the lockstep write land together, after the guards, and before `library-minting` opens the first pull request that mints library tags.

## What Changes

- **Three rulesets and two repository settings, declared in OpenTofu under `tooling/infra/`** rather than clicked into place: `protected-main` on `~DEFAULT_BRANCH` and `protected-versions` on `refs/heads/v*.x` — one pattern in place of the three `v?.…` patterns the old ruleset carried, which left `v10.1.x` unprotected — each requiring a pull request with zero approvals, the `Tests` jobs and the `version-integrity` check, merge commits only, signed commits, restricted deletions and no force pushes, with **no** bypass actor; `protected-releases` on `refs/tags/v*` and `refs/tags/*-v*`, restricting creations and updates, with the wp-botswain App as its only bypass actor, so no one hand-mints a release tag the pipeline would act on; `merge_commit_message` from `BLANK` to the pull request body, without which every multi-group summary body and `Covenant: break` trailer would be dropped at the moment of merging; and `allow_auto_merge` on, without which a bot pull request cannot merge itself.
- **Every botswain write to `main` or a series branch lands through a pull request.** The planner edits files in the working tree; the `peter-evans/create-pull-request` step already behind `steps::create_pull_request` commits them under one message with the App token (so its checks run, and so the commit is signed through the API); `xtask` enables auto-merge through the `octocrab` client it already has, so GitHub lands the pull request when the checks pass; and the tag is created on the merge commit once it exists. `release-index`'s entry follows the same path, on one pull-request branch per channel. The Contents-API write (`github::update_file`) and the read inside `write_version` lose their last callers and go. **BREAKING (contributor workflow):** nothing pushes to `main` or a series branch directly any more, including the local `merge: … into main` commits that land today.
- **Unattended merge, no cooling-off.** A bot pull request merges as soon as its required checks pass; the checks are the whole gate.
- **`cartograph` is lockstep with `src-tauri`,** prerelease included, written by every app transition in the same commit; `store` drops its `version`; the pull-request check refuses a group member whose version differs from its anchor's.
- **Two published requirements are modified** because protecting the branches contradicts them as written (below).

## Capabilities

This change stays inside the `distribution` domain. It depends on `release-groups-guards`, which creates `distribution/version-integrity` and its `version-integrity` check; `library-minting` depends on this change for the pull-request write path and the tag ruleset covering `*-v*`.

### New Capabilities

None.

### Modified Capabilities

- `distribution/release-planning`: two published requirements are `MODIFIED`.
  - _A series branch carries only cherry-picks of `main`_ admits only the planner's version commits and cherry-picks. Landing a version commit through a pull request adds a merge commit, a third kind, so the requirement is amended to allow the planner's version commit to arrive that way. Its purpose — no change originates on a series branch, so a tagged tree is self-sufficient — is unchanged.
  - _The manifest version names the channel_ ends `No other file SHALL declare a channel`, which a lockstep `cartograph` version carrying `main`'s `-nightly` prerelease would appear to violate. The requirement is amended to say that a versioned group member mirrors its anchor and is never read as a version or channel source.

  Everything else stays as published: _Release refs are minted so their pushes are observable_ is what makes the App token mandatory for the tags created on merge commits.

- `distribution/version-integrity` (created by `release-groups-guards`): two requirements are `ADDED` — _Group members declare their anchor's version_ (the lockstep check) and _The release bot's pull requests are gated like any other_ (a release-style pull request with a failing required check does not merge).

## Impact

- **`tooling/infra/`:** `github.tf` (the `integrations/github` provider authenticating as the wp-botswain App, the three `github_repository_ruleset` resources, and a `github_repository` resource importing the existing repository to hold the two settings) and the provider pin in `main.tf`; the App's id, private key, and installation id reach the provider through the 1Password environment every other provider there reads, never a literal. The App's grant gains `administration: write`.
- **`tooling/xtask`:** `planners/release.rs` writes each changed manifest into the working tree with `set_package_version` — `src-tauri/Cargo.toml` and `crates/cartograph/Cargo.toml` together — fetches and checks out the series branch it writes to, and tags the merge commit after the pull request lands; `tasks/release_index.rs` writes its entry with `Entry::render` the same way; `tools/github.rs` loses `update_file` and `file` and gains the auto-merge and merge-wait calls; `workflows/release_plan.rs` and `workflows/release_index.rs` gain `steps::create_pull_request`, the auto-merge step, and the App token scopes they need (`release-index` moves from `GITHUB_TOKEN` to the App token); the generator asserts `steps::create_release` keeps `--verify-tag` and that no workflow's App token names `administration`.
- **Manifests:** `crates/cartograph/Cargo.toml` takes `src-tauri`'s exact version; `crates/store/Cargo.toml` drops `version`; `src-tauri/Cargo.toml` gains `publish.workspace = true`.
- **Docs:** `docs/CONTRIBUTING.md` gains the merge policy — every pull request lands as a merge commit titled as its subject.
- **Release pipeline:** every transition becomes two-phase (open pull request → merge → tag); each planner job checks out the branch it writes to; the first nightly after landing records its index entry through a pull request that merged itself.

## Non-goals

- No cooling-off window before a bot pull request merges. GitHub has no time-delay rule, so a window would be a scheduled merger of our own — machinery for a veto nobody asked to use.
- No new GitHub client or transport. The bot writes reuse the pinned `create-pull-request` step and the `octocrab` calls already in `xtask`; nothing hand-rolls a commit, because signing requires creating it through the API and that step already does.
- No ruleset bypass for the App on a branch ruleset. A bypass on a checked ruleset would skip the gate that is doing all the work.
- No restriction on deleting release tags: the nightly withdraw must delete a tag it minted, under a token that cannot bypass.
- No library tags, group file, summary commits, or first-parent attribution; those are `release-groups-guards` and `library-minting`. The tag ruleset's `*-v*` pattern is declared here so the first library tag is covered the day it is minted.
- No requirement on `check-generated`: it is path-filtered and a path-filtered required check would leave a pull request waiting forever.
- No management of anything else about the repository in OpenTofu — visibility, features, collaborators, environments. The `github_repository` resource declares exactly what the plan must not drift, and nothing more.
- No re-linting of history. The rulesets apply from the moment they are active.
