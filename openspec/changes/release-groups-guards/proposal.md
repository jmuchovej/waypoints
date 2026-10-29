## Why

Bedrock, Outpost, Domain, Transload, and Waypoints are meant to version independently, but nothing in the tree says which crate belongs to which group: `the_app_declares_its_own_version` in `tooling/xtask/src/workspace.rs` records the intent without a mechanism, every library manifest still says `0.1.0`, and nothing stops a person or an agent from editing one. No check anywhere constrains what a pull request's landing message claims, who may write a version, or whether a frozen data fixture was quietly rewritten — `hk` declares no `commit-msg` hook, and no workflow reads a message at all. Commit messages are increasingly written by agents, so a version claim needs a gate in front of it, not just a well-formed header.

Why now: `main` and the `v{major}.{minor}.x` series branches are to become protected branches, landing every change as a merge commit through a pull request gated by required status checks (decided). A ruleset can only require a check that already exists and reports on every pull request, so the check has to land first, advisory, and be made required by the change that protects the branches. The importer SDK and its plugins are also in flight in the working copy, and a group file that refuses to run until they are named is the only thing that keeps them from shipping unversioned.

This is the first of four changes that together replace the earlier `release-groups` proposal. It depends on nothing; `channel-flavors` is its independent sibling; `protected-branches` requires this change, because its rulesets name the `version-integrity` check as required; and `library-minting` requires all three.

## What Changes

- **Group file.** `tooling/release-groups.toml` names every workspace crate outside `tooling/` as a versioned member of one group or an unversioned internal of one group, and declares five groups: `bedrock`, `outpost`, `domain`, `transload`, `waypoints`. `store` is the app's only internal. `tooling/xtask` and `tooling/gh-workflow` build the repository, not the product, and must not be named. Versions stay in the manifests; the file never takes a bookkeeping commit.
- **Parser and semantic checks** in `xtask`: every workspace member outside `tooling/` named exactly once, no `tooling/` member named, every anchor among its group's members, every named path present, owned paths disjoint. Both planners load and validate the file at start and refuse with the offending crate named; they do not yet attribute history by it — that is `library-minting`'s.
- **Generated schema.** `xtask` generates `tooling/schemas/release-groups.schema.json` from its own config types, and the group file binds it with a `#:schema` directive so tombi lints it, as the Tauri configs already are.
- **`version-integrity` check**, a new `xtask` task and generated workflow running on every pull request: the merge message (pull request title and description) is a conventional commit; a pull request touching more than one group's own paths is a `summary` with exactly one body header per touched group and none for an untouched one; `chore(release):` is reserved to the release bot; only the bot writes a versioned member's `[package].version`; a file under `covenant/fixtures/` is modified, deleted, or renamed only under a `Covenant: break` trailer; the committed schema is current. **Advisory at this stage:** no ruleset names it yet. `protected-branches` makes it required, and `library-minting` adds the series-pin and single-pin clauses.
- **Covenant fixture directories.** `covenant/fixtures/{bedrock,outpost,domain,transload,waypoints}/`, each with a README stating the append-only rule and the owning group; `covenant/fixtures/` excluded from treefmt so a formatter cannot rewrite one; `.git-blame-ignore-revs` created, empty, for treewide passes.
- **Docs and notes.** `docs/CONTRIBUTING.md` gains the `summary` type and its one-header-per-group body, the `Covenant: break` trailer, the rule that comma-joined regions must fall in one group, the merge policy (a pull request lands as a merge commit titled as its subject; branch commits stay conventional for the release notes). `.config/cliff.toml` gains one `commit_parsers` entry skipping `^summary:`, beside the `^chore\(release\):` skip already there.
- **Removal.** `.config/release-plz.toml`, which nothing references, goes.

## Capabilities

This change stays inside the `distribution` domain, which `nix-flake-overlay` introduced and `nix-release-index` extended with `release-planning` and `release-index`. Pull-request checks on versions and commit messages are release integrity, the front half of distribution, not toolchain reproducibility — `toolchain` deltas may never constrain product behaviour, and these checks exist precisely to constrain what a release claims.

### New Capabilities

- `distribution/version-integrity`: the pull-request checks that make every version claim verifiable before it reaches history. This change creates the capability with its purpose and four requirements — a current group-file schema, bot-only version writes, conventional and complete landing messages, and append-only covenant fixtures. `protected-branches` adds lockstep and the gating of the bot's own pull requests; `library-minting` adds the series-pin rule and the declared-versus-detected API break.

### Modified Capabilities

- `distribution/release-planning`: gains one `ADDED` planner-side invariant — every workspace crate outside `tooling/` is named in the group file, and a planner refuses to run until it is. No published requirement is modified here; the two amendments protecting the branches requires belong to `protected-branches`.

## Impact

- **New files:** `tooling/release-groups.toml`; `tooling/schemas/release-groups.schema.json`; `covenant/fixtures/{bedrock,outpost,domain,transload,waypoints}/README.md`; `.git-blame-ignore-revs`; `.github/workflows/version-integrity.yaml` (generated).
- **`tooling/xtask`:** a `release_groups` module (parser deriving `schemars::JsonSchema`, semantic checks, `cargo xtask release-groups schema`); `planners/release.rs` and `planners/nightly.rs` load and validate the file at start; a `version_integrity` task under `src/bin/xtask/tasks/` and its library module; a new `workflows/version_integrity.rs` generator; `schemars` joins `xtask`'s dependencies.
- **Formatting and notes:** `.config/treefmt.toml` excludes `covenant/fixtures/`; `.config/cliff.toml` skips `^summary:`.
- **Docs:** `docs/CONTRIBUTING.md`, as above.
- **Removed:** `.config/release-plz.toml`.
- **Contributor workflow:** nothing changes yet. The check reports on every pull request and blocks none until `protected-branches` requires it; a hand-edited version or a free-form title shows up as a failed advisory check from the day this lands.

## Non-goals

- No attribution of history by group, no summary reading, no severity, no minting, no floors, no pins: the planners load the group file and refuse on a malformed one, nothing more. All of that is `library-minting`.
- No lockstep check, no rulesets, no repository settings, no bot pull requests, no routing of the planners' direct writes. That is `protected-branches`; this change is what it requires first.
- No API-compatibility check (`cargo-semver-checks`) and no series-pin or single-pin clause in the check; `library-minting` adds them.
- No flavor work. `channel-flavors` is independent of this change.
- No release-please or release-plz; `xtask` is extended instead, and the orphaned `.config/release-plz.toml` is removed.
- No new conventional-commit type for release bookkeeping. `chore(release):` is what the planner already writes and what `.config/cliff.toml` already skips; only `summary` is added.
- No covenant fixture coverage. This change ships the directories and the append-only rule. A fixture per migration, payload type, ACL record, and envelope version is a follow-up touching `bedrock`, `outpost`, and `store`.
- No versioning for components that do not ship yet; each becomes its own group when it does. The importer SDK and its plugins, in flight in the working copy and not yet on `main`, must be named in the group file when they land, and every planner run refuses until they are. Which group they join is an open question in design.md.
- No re-linting of history. The check applies to pull requests from the landing of this change.
