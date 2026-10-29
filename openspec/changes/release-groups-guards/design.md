## Context

See proposal.md — Why. Every claim below about current behaviour names its file and says what kind of fact it is: _implemented_ (in code), _designed_ (in an open change), _decided_ (stated, not yet in code), or _proposed_ (this change). Verified 2026-10-29 against `4006732e`; every commit since `9541d4d2` touches only `openspec/`, so the tree outside it is the one the earlier `release-groups` proposal was checked against — with one correction: `xtask`'s planners live under `tooling/xtask/src/planners/` and `releases/`, not the `tasks/` paths that proposal cites.

| Fact                                                                                                                                                                                                                                                                                                                                                                                                                                              | Where                                                                                                                                                                                          | Status                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Both planners read one `git log`: `--format=%s --reverse <range> -- . ':(exclude)nix/releases/'`. Subjects only, every commit in range; bodies and trailers are invisible. Their jobs check out full history, and a workflow test holds every planner job to it.                                                                                                                                                                                  | `tooling/xtask/src/releases/index.rs` (`PRODUCT_PATHSPEC`, `product_commits`, `fixture::Repo`), `workflows.rs` (`planners_check_out_the_history_they_walk`)                                    | implemented — unchanged here; `library-minting` makes the walk group-aware   |
| Only `cut` reads history; `plan` reads the app version from `cargo metadata --no-deps`. Neither planner knows a group.                                                                                                                                                                                                                                                                                                                            | `tooling/xtask/src/planners/release.rs` (`plan`), `planners/nightly.rs`, `workspace.rs` (`load`, `version`, `the_app_declares_its_own_version`)                                                | implemented                                                                  |
| Every message the planner writes is `chore(release): …`, which `.config/cliff.toml` skips, and a test holds the planner to it.                                                                                                                                                                                                                                                                                                                    | `planners/release.rs` (`RELEASE_PREFIX`, `commit_message`, `next_main_message`, `every_message_the_planner_writes_is_conventional_and_skipped`), `releases/index.rs` (`commit_message`)        | implemented — `baf7e109`                                                     |
| Fourteen workspace members on `main`: the three `bedrock*`, four `outpost*`, `cartograph`, `domain`, `store`, `transload`, `src-tauri`, `tooling/gh-workflow`, `tooling/xtask`. Every library version is an explicit `0.1.0`; `publish = false` is inherited by every member but `src-tauri`. `bedrock-derive` is a proc-macro crate depending on no workspace crate. The working copy adds `crates/importer-sdk` and three `plugins/importer-*`. | `Cargo.toml`, member manifests                                                                                                                                                                 | implemented                                                                  |
| `planners/test.rs` already maps a changed path to the workspace crate whose manifest directory is its longest prefix, over `package_directories` from `cargo metadata`.                                                                                                                                                                                                                                                                           | `tooling/xtask/src/planners/test.rs` (`owning_package`, `package_directories`, `default_base`, `changed_files`)                                                                                | implemented                                                                  |
| Eight workflows, all generated by `xtask`: `Tests` on every pull request (`branches: **`, test-plan scoped, `fetch-depth: 0` for its merge base), `check-generated` (path-filtered), three release workflows, `release-plan`, `release-index`, `devenv-update`. A job runs `cargo xtask …` through `steps::cargo_xtask` under the `xtask` mise lane. No commit-message lint exists anywhere, and `hk` declares no `commit-msg` hook.              | `tooling/xtask/src/workflows.rs`, `workflows/pull_request.rs`, `workflows/check_generated.rs`, `workflows/_jobs.rs` (`test`, `mise_env`), `tools/mise.rs` (`MiseEnv::Xtask`), `.config/hk.pkl` | implemented                                                                  |
| The release bot is the `wp-botswain[bot]` GitHub App; its pull requests, where a workflow opens one, carry that login as author.                                                                                                                                                                                                                                                                                                                  | `tooling/xtask/src/workflows/_steps.rs` (`app_token`, `botswain_credentials`), `workflows/devenv_update.rs`                                                                                    | implemented                                                                  |
| The repository allows all three merge methods; `merge_commit_title` is `PR_TITLE` but `merge_commit_message` is `BLANK`, so a merge commit carries no body today.                                                                                                                                                                                                                                                                                 | `gh api repos/jmuchovej/waypoints`                                                                                                                                                             | implemented — `protected-branches` moves the setting (D3)                    |
| `main` and the `v{major}.{minor}.x` branches are to be protected: every change lands as a merge commit through a pull request gated by required status checks; the merge message is the pull request's title and description.                                                                                                                                                                                                                     | per John                                                                                                                                                                                       | decided — the rulesets are `protected-branches`'s; this change is their gate |
| tombi formats every `*.toml` except the generated mise files; schemas bind by an in-file `#:schema` directive (Tauri configs) or a `[[schemas]]` entry; treefmt's `excludes` list is explicit, and `oxfmt` formats every `*.md` but `website/`.                                                                                                                                                                                                   | `.config/tombi.toml`, `.config/treefmt.toml`                                                                                                                                                   | implemented                                                                  |
| Release notes come from `git-cliff --latest` in the release workflows. `.config/cliff.toml` sets `require_conventional = true`, `filter_unconventional = true`, skips `^chore\(release\):`, and filters merge commits out of the rendered body. It knows no `summary` type.                                                                                                                                                                       | `workflows/_steps.rs` (`release_notes`), `.config/cliff.toml`                                                                                                                                  | implemented                                                                  |
| `docs/CONTRIBUTING.md` fixes the type table (`feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `style`, `build`, `ci`, `chore`) and the regions; `release` and `deps` are special scopes; a rare multi-region change comma-joins regions; `!` plus a `BREAKING CHANGE:` footer marks a break. It names no `summary` type, no `Covenant` trailer, and no merge policy.                                                                            | `docs/CONTRIBUTING.md`, `AGENTS.md`                                                                                                                                                            | implemented (docs)                                                           |
| `.config/release-plz.toml` is referenced by nothing (`rg release-plz` outside `openspec/` finds nothing); the working copy already deletes it.                                                                                                                                                                                                                                                                                                    | `.config/release-plz.toml`                                                                                                                                                                     | implemented (orphaned)                                                       |
| `.git-blame-ignore-revs`, `covenant/`, `tooling/release-groups.toml`, `tooling/schemas/`, and a `schemars` dependency do not exist.                                                                                                                                                                                                                                                                                                               | —                                                                                                                                                                                              | proposed                                                                     |

## Goals / Non-Goals

**Goals:**

- One declarative file names every product crate, and both planners refuse to run on a tree it does not describe.
- Every version claim a pull request makes is checked before it reaches history: who wrote the version, whether the landing message is conventional and, for a multi-group change, complete, and whether a frozen fixture was rewritten undeclared.
- The check exists and reports on every pull request before any ruleset names it, so `protected-branches` can require it on the day it lands.
- Everything is derivable: the schema from the parser, the workflow from `xtask`, the group of a path from the group file.

**Non-Goals (design-level, beyond the proposal's):**

- No change to how a pull request's _branch_ commits are written; only the merge message is checked.
- No second source of versions: no ledger file, no version field in the group file.
- No reading of the group file by the planners beyond validation. The keys whose semantics belong to later changes are parsed and schema-checked here so the file's shape is settled once; nothing in this change acts on them.
- No new conventional-commit type where an established one serves. The bookkeeping marker stays `chore(release):`.

## Decisions

D1–D6 were settled on 2026-10-11 in the earlier `release-groups` proposal and revised where dated; their numbers there are given in parentheses. D7 and D8 need sign-off (**AUDIT**).

### D1 (2026-10-11, revised 2026-10-29): One declarative file names every product crate; versions live only in manifests

Monolith D1.

- **Why:** release-please's useful idea is a declarative group table. Keeping versions out of it means the file never takes bookkeeping commits, and the manifest stays the sole version source, as `distribution/release-planning` requires.
- **Location:** `tooling/release-groups.toml`; shape in the Appendix.
- **Groups:** `bedrock`, `outpost`, `domain`, `transload`, and `waypoints`. `domain` is its own group because its data structures are the core of the data covenant. `transload` replaced `importer` as a workspace member and is a library the app ships, so it is a group rather than an internal (revised 2026-10-29).
- **The app:** `src-tauri` and `cartograph` are versioned members, since `cartograph` is the API surface. `store` is an unversioned internal, the app's only one. Dropping `store`'s `version` field and bringing `cartograph` into lockstep with `src-tauri` are `protected-branches`'s, where the lockstep check that would notice them lands.
- **Tooling is not named.** `tooling/xtask` and `tooling/gh-workflow` build the repository, not the product, and the file refuses to name them (revised 2026-10-29; the earlier draft listed `xtask` as `ungrouped`).
- **Exhaustive:** every other workspace member is named, as a versioned member or an internal. The importer SDK and its plugins, in flight in the working copy, will block every planner run the day they land until the file names them — which is the gate working.
- **Where it is read:** `planners/release.rs::plan` and `planners/nightly.rs` load and validate the file before anything else, so a malformed file refuses a dry run too. Neither attributes history by it yet; `library-minting` makes `releases/index.rs::product_commits` group-aware.
- **Rejected:** a version ledger like `.release-please-manifest.json` (a second source of truth); groups hard-coded in Rust (drifts silently from workspace members).
- **Gate:** an unnamed, doubly named, or `tooling/` crate makes the planner refuse; the file validates against its schema (D6).

### D2 (2026-10-11): Extend xtask; adopt neither release-please nor release-plz

Monolith D2.

- **Why:** release-please releases by merging a PR on one target branch. It does not model cut → `-pre.N` → promote across `v{minor}.x`, and its Rust strategy [fails on virtual workspaces](https://github.com/Gitlawb/node/issues/130). release-plz ([config](https://release-plz.dev/docs/config)) is closer and uses the same `next_version` crate, but its `version_group` bumps only members that changed or depend on a change, and `bedrock-derive` depends on no workspace crate, so it would drift.
- **Borrowed, not adopted:** release-please's multiple-changes-per-commit bodies (D3) are kept as a convention inside `xtask`; its release PR is `protected-branches`'s and `library-minting`'s.
- **Consequence:** `.config/release-plz.toml`, which nothing references, goes with this decision.
- **Rejected:** release-please for all groups; release-plz for libraries only.
- **Gate:** none; this is a scoping decision.

### D3 (2026-10-11, revised 2026-10-19): A commit touching more than one group is a summary

Monolith D5, the format half. How each group reads its own header for severity is `library-minting`'s.

- **Why:** one header, many groups. Rather than parse multi-scope subjects, a multi-group commit's subject summarizes, and its body carries one header per touched group. This is the convention release-please uses for [multiple changes in one commit](https://github.com/googleapis/release-please#what-if-my-pr-contains-multiple-fixes-or-features).
- **Format:** subject type `summary` (reserved for this). Body headers are scoped by group name, such as `feat(bedrock)!: rename Filter::eq` and `refactor(outpost): follow the rename`.
- **Where it lives:** the merge commit. The decided merge policy makes its message the pull request's title and description, so the title is the subject and the description carries the per-group headers and any `Covenant: break` trailer. Branch commits underneath are never checked by the gate; they stay conventional all the same, for the reason D7 gives.
- **What the check reads today:** the pull request's title and body from the event payload, and its diff against the merge base, as `planners/test.rs::default_base` already computes. That is the message the merge commit _will_ carry once `protected-branches` sets `merge_commit_message` to the pull request body; until then a landed merge commit carries no body, which nothing reads yet.
- **Completeness is checked against the diff:** a pull request changing more than one group's own paths must be a summary with exactly one header per touched group, and a header for a group the diff does not touch is refused too. The group of a path is the group file's, through the longest-prefix rule `planners/test.rs::owning_package` already applies to crates.
- **Single-group commits** keep free-form scopes; the path decides the group. CONTRIBUTING's comma-joined regions stay legal only while every region falls in one group — `refactor(app,store): …` is one `waypoints` commit; `refactor(domain,cartograph): …` must be a summary.
- **Rejected:** multi-scope subjects such as `feat(bedrock,outpost)!:`; an unscoped `!` applying to every touched group.
- **Gate:** tests for a two-header summary, a summary missing a touched group (refused), a summary naming an untouched group (refused), and a single-group commit with a free-form scope.

### D4 (2026-10-11): Covenant fixtures are append-only and owned by a group

Monolith D8, the directories and the append-only rule. The floors a fixture change puts under a cut are `library-minting`'s.

- **Layout:** `covenant/fixtures/<group>/`: `domain` (data structures), `outpost` (envelope and ACL records), `bedrock` (storage format), `transload` (import corpora), `waypoints` (migrations).
- **Why append-only:** modifying, deleting, or renaming a fixture changes what old data means. A path rule cannot be defeated by a message-format change, the same reasoning that put the index exclusion in the planners' walk.
- **Fixtures are data, not tests.** `covenant/fixtures/` is a repo-root corpus of frozen inputs; the tests that read them are ordinary `cargo test` cases in the owning crate. What a test cannot do is notice its own fixture being rewritten; the path rule covers that at pull-request time: append-only unless the message carries `Covenant: break`.
- **Formatting never stands alone.** A formatter's changes amend the commit they belong to. Treewide passes are listed in `.git-blame-ignore-revs`, created empty here so the planners' exclusion in `library-minting` has a file to read. Fixtures are excluded from treefmt, since an amended reformat would trip the append-only check.
- **Scope here:** the directories, the rule, the trailer, the exclusion, and the ignore-revs file. Coverage is a follow-up.
- **Gate:** check tests for add (pass), modify without trailer (fail), and modify with trailer (pass); treefmt leaves a fixture untouched.

### D5 (2026-10-11): Only botswain writes versions, and `chore(release):` is its subject

Monolith D9, the bot-only write and the reserved subject. The lockstep half — every member equals its anchor — is `protected-branches`'s, where `cartograph` is first brought into lockstep.

- **Why:** if people or agents edit versions, the planner's output stops being the record.
- **Checks:** a diff to any versioned member's `[package].version` by an author other than `wp-botswain[bot]` fails, naming the file; a `chore(release):` subject by any other author fails. The author is the pull request's `user.login` in the event payload.
- **Why reserve the subject too:** `.config/cliff.toml` skips `^chore\(release\):` and `library-minting` will exclude it from attribution, so a person writing it would make a product change invisible to both the notes and the planners.
- **Gate:** check tests for both; the app-version test in `workspace.rs` stays.

### D6 (2026-10-11): The group file's schema is generated from the parser and committed

Monolith D11.

- **Why:** tombi lints TOML against JSON Schema, bound by an in-file `#:schema` directive or a `[[schemas]]` entry ([tombi docs](https://tombi-toml.github.io/tombi/docs/json-schema/)). Generating the schema from the parser's types means the two cannot drift.
- **Shape:** the config types derive `schemars::JsonSchema`; `cargo xtask release-groups schema` writes `tooling/schemas/release-groups.schema.json`. Doc comments become hover text, and enums become closed sets.
- **Binding:** an in-file directive, `#:schema ./schemas/release-groups.schema.json`, the Tauri-config precedent. Tombi resolves a relative path against the TOML file's own directory, so the path is stable wherever the lint runs from. The file is hand-maintained, so the binding belongs with it rather than in `.config/tombi.toml`, which binds schemas only to hand-edited mise configs.
- **Split of duties:** the schema checks shape; `xtask` checks semantics (every crate named, paths disjoint, paths exist). The `version-integrity` check regenerates the schema and compares it with the committed file, so a parser change cannot land without its schema.
- **Gate:** a test regenerates the schema and compares it with the committed file; `tombi lint` fails on a misspelled key.

### D7 (2026-10-17, proposed — AUDIT): Release notes keep reading branch commits; summary-aware notes are a follow-up

Monolith D15.

- **Why:** `git-cliff --latest` renders the release notes, with `require_conventional = true` and merge commits filtered out of the body. Under D3 the merge commit carries the summary and the branch commits beneath it are checked by nothing — so they could in principle go free-form, and then the notes step would fail on the first unconventional one, which is how the first cut lost its draft. Pointing git-cliff at merge commits instead would duplicate every conventional branch commit under its merge.
- **Shape:** one `commit_parsers` entry skipping `^summary:`, beside the `^chore\(release\):` skip already there. The per-group headers live in a body, which cliff does not parse as subjects, so they need no entry of their own. Branch commits stay conventional per CONTRIBUTING, which the notes already depend on; the gate does not check them.
- **Rejected:** teaching git-cliff to walk first-parent and expand summary bodies — the right end state, but it is release-notes work with its own design, and nothing here depends on it.
- **Gate:** `git-cliff --latest` renders notes for a merge-landed pull request whose branch commits are conventional, and renders no entry for a `summary:` merge commit.

### D8 (2026-10-29, proposed — AUDIT): The check runs on every pull request, advisory until `protected-branches` requires it

No monolith number; this decision is the split's.

- **Why:** a ruleset can name only a check that exists and reports on every pull request, and a required check that fails to report leaves a pull request waiting forever. So the check lands first, with no ruleset naming it, and `protected-branches` requires it in the same change that protects the branches. Nothing about the check changes when it is flipped; the design here is for the required state.
- **Shape:** `cargo xtask version-integrity`, one Linux job in a generated `version-integrity` workflow triggered on `pull_request` for every branch with no path filter, running under the `xtask` mise lane as the planners do, checking out full history so the merge base its diff needs resolves — the `Tests` job's shape in `_jobs.rs::test`. Its inputs are the event payload (`GITHUB_EVENT_PATH`: title, body, author) and `git diff` against the merge base; it needs no token and no API call.
- **What it checks here:** the five clauses the Appendix's `[enforce]` table names — conventional message, complete summary, bot-only `chore(release):`, bot-only version writes, append-only fixtures — and the schema's currency (D6). Lockstep is `protected-branches`'s; series pins, single pins, and the API break are `library-minting`'s; each adds its clause and its `[enforce]` key.
- **Advisory means ignorable, briefly.** Until the ruleset names it a failed check blocks nothing, so a hand-edited version could still land by someone merging over red. Accepted: the window closes with `protected-branches`, and the alternative — landing the check and the rulesets together — is what the earlier proposal's own review found to be the wrong order.
- **Rejected:** a `commit-msg` hook in `hk` (checks the branch commits, which D3 and D7 deliberately leave unchecked, and runs only on machines that installed it); landing the check already required (nothing to require it with).
- **Gate:** `cargo xtask workflows` writes `.github/workflows/version-integrity.yaml`; the workflow runs and reports on a pull request with a free-form title; `GET /rules/branches/main` still lists no required check.

## Risks / Trade-offs

- [A path-filtered required check never reports] → `version-integrity` triggers on every pull request with no path filter (D8); `check-generated` stays path-filtered and is never required.
- [The merge commit carries no body today] → the check reads the pull request's title and body, which are what the merge commit will carry once `protected-branches` sets `merge_commit_message`; until then nothing reads a landed body (D3).
- [Advisory check merged over] → the window lasts until `protected-branches` lands; the check's output is the same either way, so there is nothing to redo (D8).
- [Formatter rewrites a fixture] → formatting amends the commit it belongs to, treewide passes go in `.git-blame-ignore-revs`, and fixtures are excluded from treefmt (D4).
- [Summary bodies drift from the diff] → the gate requires exactly one header per touched group, no more and no fewer (D3).
- [The group file's keys outrun this change] → keys whose semantics belong to later changes are parsed and schema-checked so the shape is settled once; the semantic checks here act only on members, internals, anchors, and paths. A later change that reads a key changes no file layout.
- [The importer SDK and plugins block every planner run the day they land] → intended; the planners name the crates, and the Open Questions record the choice to make.
- [Pre-existing history does not meet the check] → the check applies to pull requests from its landing; the 7 local `merge: … into main` commits and 4 non-conventional subjects in first-parent history are not re-linted, and the planners read older commits as `next_version` does today.
- [`tombi lint` runs somewhere the job does not] → the schema-currency test in `xtask` holds regardless; whether the job also runs `tombi lint` is an open question that changes no spec.

## Migration Plan

1. **Group file, parser, schema, and the planners' load** (D1, D6): landable with no change to any planner decision; from here on a workspace member the file does not name refuses every planner run, dry runs included.
2. **The check and its workflow** (D3, D5, D8, D4's check): advisory; it reports on every pull request from the first run.
3. **Fixture directories, the treefmt exclusion, and `.git-blame-ignore-revs`** (D4).
4. **CONTRIBUTING, the cliff skip, and the release-plz removal** (D2, D7).
5. `protected-branches` then names the check as required; `library-minting` then adds its clauses.

Rollback: every step is additive. Removing the workflow generator and regenerating takes the check away; the group file and its parser can stay in place harmlessly, since nothing but validation reads them.

## Open Questions

- Which group the importer SDK and its plugins join when they land. The SDK reads as a library the app ships (its own group, or part of `transload`); the plugins may be internals of that group or ship separately. The planner refuses until the file says, so this cannot be forgotten, only deferred.
- Whether `tombi lint` runs in the `version-integrity` job or only through `treefmt`'s tombi formatter step. The schema-currency test in `xtask` holds either way.

## Appendix: `tooling/release-groups.toml` sketch

The shape the parser in task 1.1 reads. Field names are proposals; the generated schema (D6) is what tombi enforces. Keys this change does not act on — `breaking`, `floors`, `minted`, `owns`, `counts`, `features-always-increment-minor`, `channels`, `flavors` — are parsed and shape-checked here so the file is settled once; `channel-flavors` and `library-minting` give them their meaning, and `protected-branches` and `library-minting` add their own `[enforce]` keys.

```toml
#:tombi toml-version = "v1.1.0"
#:schema ./schemas/release-groups.schema.json

# Every workspace crate outside tooling/ is named: as a member or an internal.
# tooling/ builds the repository, not the product, and must not appear.

# Never product work, for any group or planner.
[bookkeeping]
  paths        = [ "nix/releases/" ]
  ignore-revs  = ".git-blame-ignore-revs"
  bot-subjects = [ "chore(release):" ]   # reserved to botswain

[group.bedrock]
  members  = [ "crates/bedrock", "crates/bedrock-core", "crates/bedrock-derive" ]
  anchor   = "crates/bedrock"           # every member == anchor (lockstep)
  tag      = "bedrock-v{version}"
  breaking = "header"                   # `!` in this group's own header
  floors   = [ "cargo-semver-checks", "covenant-fixtures" ]  # bedrock-derive: declared-only
  fixtures = "covenant/fixtures/bedrock/"
  minted   = "cut"                      # on main only; a series pins, never builds in-tree

[group.outpost]
  members  = [ "crates/outpost", "crates/outpost-fabric", "crates/outpost-hold", "crates/outpost-protocol" ]
  anchor   = "crates/outpost"
  tag      = "outpost-v{version}"
  breaking = "header"
  floors   = [ "cargo-semver-checks", "covenant-fixtures" ]
  fixtures = "covenant/fixtures/outpost/"   # envelope, ACL records
  minted   = "cut"
  # The wire (ALPN outpost/sync/v1 + session negotiation) is not this number.
  # A wire break is an interop break: it needs the covenant trailer.

[group.domain]                          # core of the data covenant
  members  = [ "crates/domain" ]
  anchor   = "crates/domain"
  tag      = "domain-v{version}"
  breaking = "header"
  floors   = [ "cargo-semver-checks", "covenant-fixtures" ]
  fixtures = "covenant/fixtures/domain/"
  minted   = "cut"

[group.transload]
  members  = [ "crates/transload" ]
  anchor   = "crates/transload"
  tag      = "transload-v{version}"
  breaking = "header"
  floors   = [ "cargo-semver-checks", "covenant-fixtures" ]
  fixtures = "covenant/fixtures/transload/"   # import corpora
  minted   = "cut"

[group.waypoints]
  members   = [ "src-tauri", "crates/cartograph" ]   # lockstep; cartograph is the API surface
  internals = [ "crates/store" ]                      # owned and counted, never versioned
  anchor    = "src-tauri"
  tag       = "v{version}"
  owns      = "remainder"   # everything no other group owns, minus tooling/ and bookkeeping
  counts    = "closure"     # its own paths plus every group's: the binary ships them
  breaking  = "trailer:Covenant: break"
  features-always-increment-minor = true
  floors    = [ "covenant-fixtures" ]   # any group's fixtures, plus its own
  fixtures  = "covenant/fixtures/waypoints/"   # migrations
  minted    = [ "cut", "advance", "promote", "patch" ]
  channels  = [ "nightly", "preview", "stable" ]
  flavors   = [ "nightly", "preview", "stable", "development" ]   # declared by every group crate

[enforce]                               # pull-request checks this change carries
  commits        = "conventional"
  summary-type   = "summary"            # one header per touched group in the body
  version-writes = "botswain"
  fixtures       = "append-only"
  schema         = "current"
```
