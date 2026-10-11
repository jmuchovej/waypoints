## Context

See proposal.md — Why. The facts that shape the approach, all read from the tree on 2026-10-10:

- **The fork.** `waypoints-hq/gh-workflow` at `e4f0289` is upstream `tailcallhq/gh-workflow` plus one commit: `fluent.rs` and `outputs.rs` added, `job.rs` and `lib.rs` touched, 186 insertions, no deletions — all in `crates/gh-workflow`. The checkout has no `upstream` remote configured. Three crates: `gh-workflow`, `gh-workflow-macros` (proc-macro, `syn = 3`, untouched by the fork since upstream's version-alignment commit), `gh-workflow-tailcall` (unused here).
- **`gh-workflow-macros` 0.9.0 is published.** On crates.io since 2026-07-22, depending on `heck ^0.5`, `quote ^1`, `syn ^3`. The fork's `gh-workflow` already names it as `{ path = ..., version = "0.9.0" }`, so the registry crate satisfies the same requirement.
- **How it is consumed.** `Cargo.toml` names it as `{ git = ..., rev = "e4f02895aa72", version = "0.9.0" }` in `workspace.dependencies`; `tooling/xtask` is the only consumer.
- **The vendor precedent.** `vendor/specta-macros` and `vendor/specta-serde` are flat copies wired through `[patch.crates-io]`, each with a README naming the upstream version and the patch. They are not workspace members. `.github/renovate.json5` ignores `**/vendor/**`.
- **Workspace conventions that bite.** `serde_yml`, `derive_setters`, `merge`, `strum_macros`, `async-trait`, and `insta` are not declared at the root. `syn` is `2` at the root for `bedrock-derive`; `syn` 3.0.6 already coexists in `Cargo.lock` as the macros crate's private dependency. `workspace.lints.clippy` differs from the fork's (`use_self = warn`). `treefmt` formats every `*.rs` in the tree; `prek` runs clippy, cargo-check, cargo-test, treefmt.
- **Where third-party code lives.** `vendor/` holds patched application dependencies (the two specta crates). `tooling/` holds development tooling (`xtask`, `icongen`, `bin/`).
- **jj does not support git submodules.** A colocated checkout would leave the submodule path empty under jj, and `cargo` would fail to find the crate. That rules the submodule out regardless of other merits.
- **The renderer already post-processes text.** `WorkflowSource::render` in `workflows.rs` calls `serde_yml::to_string`, prepends the family's disclaimer, and trims trailing whitespace line by line.
- **zizmor's inline grammar.** `# zizmor: ignore[rule]`, optionally followed by free text, as a YAML comment on any line within the finding's span. The `cache-poisoning` audit's primary location is the `uses:` line, and it adds the whole step body as a hidden location, so a comment on the step's `id:` line is inside the span. Comments inside strings or block scalars are not recognised.
- **Where zizmor runs.** The `zizmor` hook in `.config/hk.pkl`, locally. No workflow runs it today.

## Goals / Non-Goals

**Goals:**

- A library edit is one commit here, and the library is held to the same hooks as everything else.
- A step owns its suppressions; the rendered YAML carries them where zizmor reads them; nothing at file scope.
- zizmor becomes the authority on third-party action risk, in CI, and the generator's tests stop imitating it.

**Non-Goals:**

- Those in the proposal.
- Preserving upstream git history inside this repository.
- Making the renderer a general comment-capable serializer.

## Decisions

### D1 — One crate in `tooling/`, as a workspace member, synced by a recipe; the macros crate from crates.io

**Chosen** (2026-10-10). `tooling/gh-workflow/` is copied from the fork's `crates/gh-workflow` at `e4f0289` (source, manifest, snapshot fixtures). `Cargo.toml` replaces the git dependency with `gh-workflow = { path = "tooling/gh-workflow" }`, adds the crate to `members`, and adds `gh-workflow-macros = "0.9.0"` to `workspace.dependencies`. A `tooling/gh-workflow/README.md` records upstream repository, upstream revision, the local delta (today: the `FluentBuilder` / `NamedJob` / outputs commit; after this change: the suppression field as well), and the macros version the source was written against. `just gh-workflow-sync <rev>` fetches `tailcallhq/gh-workflow` at `<rev>` into a scratch directory and copies `crates/gh-workflow` over the in-tree one; the maintainer then uses `jj diff` and `jj resolve` to re-apply the delta and updates the README's revision.

**Why the macros crate is not brought in.** The fork never touched it; the published 0.9.0 is what the fork's `gh-workflow` already declares a dependency on by version; and leaving it in the registry keeps its `syn` 3 a private proc-macro dependency, which is how the lockfile resolves today. Bringing it in-tree would have forced a choice between a workspace-wide `syn` bump that `bedrock-derive` has to survive and a second `syn` key in the workspace table — a cost with nothing on the other side.

**Why `tooling/` rather than `vendor/`.** `vendor/` is for patched application dependencies — code that ships in the product's dependency graph and is waiting on an upstream fix. gh-workflow is development tooling: it exists to generate CI, sits beside `xtask`, and is edited as part of the generator. It is third-party in origin and first-party in use, and the directory should follow the use. (An earlier draft chose `vendor/` for its README-and-Renovate-ignore convention; the README convention carries over, and the Renovate point is moot once the crates are workspace members managed through the workspace table.)

**Why a workspace member, like `xtask`.** gh-workflow is live — this change itself extends it — so it is compiled, linted, tested, and formatted by `prek` like the generator that uses it, and it inherits its dependencies from `workspace.dependencies` like every other member. The one-time cost is a reformat under this repository's `rustfmt` settings and whatever clippy raises under its lints; both are absorbed in the import commit.

**Alternatives rejected.**

- _Git submodule._ Unusable under jj (see Context). Would otherwise be the conventional answer.
- _`git subtree add --prefix tooling/gh-workflow`._ Keeps upstream history mergeable with `git subtree pull`, and works in a colocated repo because the result is ordinary commits. Rejected because a subtree is the whole upstream tree — `gh-workflow-tailcall`, upstream's `.github`, Renovate config, release tooling — and every pull re-adds what was pruned. For a 186-line delta, merge history buys little over a diff.
- _Pristine upstream plus a patch series (`patches/*.patch`, applied by a recipe)._ Keeps upstream untouched and the delta explicit. Rejected because it is the opposite of "straightforward to edit": every change is made to the source, then exported to a patch, then re-applied to verify. This is the form a Nix derivation would want, and nothing here is built by Nix.
- _Keep the git dependency and the fork checkout._ The status quo being replaced.

### D2 — Every dependency is inherited from the workspace

The manifest uses `workspace = true` for every dependency, including `gh-workflow-macros`, and `[lints] workspace = true`, exactly as `xtask` does. `workspace.dependencies` gains `gh-workflow-macros = "0.9.0"` and the six the fork pins that the root lacks — `serde_yml`, `derive_setters`, `merge`, `strum_macros`, `async-trait`, `insta` — at the fork's versions, which are already what `Cargo.lock` resolves. No existing workspace version moves.

The one coupling to watch: the in-tree `gh-workflow` source is written against a specific macros API. Within 0.9.x that is stable by semver; a Renovate bump to 0.10 lands as an ordinary pull request whose `cargo check` says whether the source still compiles. The README names the macros version the source was taken alongside, so the pairing is not tribal knowledge.

_Alternative rejected:_ crate-local pins for the mismatched dependencies. Makes this the only member whose versions are not the workspace's, which is the kind of exception Renovate and a reader both have to learn about.

### D3 — The suppression travels on the step, skipped by serde, placed by the renderer

`StepValue` gains `#[serde(skip)] pub lint_ignores: Vec<LintIgnore>` where `LintIgnore { rule: String, reason: String }`, and `Step<A>` gains `.zizmor_ignore(rule, reason)`. Before serializing, `render` walks `workflow.jobs` and collects `(job name, step id, Vec<LintIgnore>)` from the typed structure. After serializing and prefixing the disclaimer, a pass finds, within each job's block, the line `- id: <id>` or `id: <id>` at step indentation and appends ` # zizmor: ignore[rule] reason` (several rules comma-joined in one comment). A collected directive with no id, or whose id line is not found exactly once, returns an error naming the job and step; `write` never runs.

**Why the `id:` line.** It is the one field every step we care about can be given, it is unique within a job, it serializes first in `StepValue` (so it is the step's opening `- ` line), and it is inside the finding's hidden span. `uses:` would be the primary span but is absent on `run:` steps; `name:` is free text.

**Why skip rather than a side table.** The directive belongs to the step that needs it; `rust_cache()` declares it where its doc comment already explains it. A side table in xtask keyed by id would separate the reason from the step and go stale on a rename without the compiler noticing. The field is a fork change, but the fork is now in-tree, which is the point.

**Alternatives rejected.**

- _Encode the directive in `name:` and rewrite the quoted scalar._ `#` forces serde_yml to quote the string; moving the comment outside the quotes is a rewrite of the scalar's quoting, fragile and specific to one serializer's quoting rules.
- _zizmor's config with `file.yaml:line:col` ignores._ Supported, but line numbers in generated files move on every regeneration.
- _Switch serializers._ No maintained Rust YAML serializer emits comments from serde data; `serde_yml` is what the library and its snapshot tests are written against.

### D4 — `rust_cache()` declares the one suppression this repository needs today

`.id("rust-cache").zizmor_ignore("cache-poisoning", "written only from main (save-if), restored by test jobs that publish nothing")`. The reason is the doc comment's, shortened. The build jobs declare one suppression of their own once `r2-release-build-cache` has landed: `setup_mise` with `cache: true` carries `cache-poisoning` with the reason recorded there (mise's install cache under GitHub's default write rules, tools pinned by `mise.lock`). Everything else in a build job is a `run:` step zizmor does not classify as a cache.

### D5 — The substring clause leaves the generator test; the positive assertions stay

`environments_follow_the_vault_credential` keeps: credential and environment come together; provider and credential belong on the step; and — from `r2-release-build-cache` D7 — a step setting `RUSTC_WRAPPER` follows a server start resolving the `ci` profile's `cache` scope, every `cache-restore-*` / `cache-save-*` step resolves that scope itself, no step's `env` names the store credential, and no `ci` resolution is unscoped. The clause rejecting `uses` containing `cache` or `with.cache == true` is deleted with its comment, and with it that change's by-name allowance for `mise-action`'s cache, which exists only to get past the clause. The comment that remains points at zizmor for third-party actions. If this lands before `r2-release-build-cache`, the clause is deleted anyway; the positive assertion arrives with that change.

### D6 — zizmor in CI, taking the `hk` job when it exists

The requirement is that zizmor runs over the generated workflows on every pull request. If the planned `hk check` job has landed in `pull_request.rs` by the time this is implemented, it already covers `.github/workflows/*.yaml` through the `zizmor` hook and nothing more is needed. If not, `pull_request.rs` gains a job running `zizmor .github/workflows` from the `devtools` mise lane, to be folded into the `hk` job when that lands. `.github/zizmor.yml` is deleted once its `cache-poisoning` block is removed and nothing else remains.

## Risks / Trade-offs

- [Reformatting the in-tree source under this repository's rustfmt settings makes future upstream diffs noisier] → One-time, and accepted; `just gh-workflow-sync` is followed by `jj fix`, so upstream's formatting never persists and the diff against the previous in-tree state is what matters.
- [Clippy under the root's lint table raises warnings in the imported code] → Fixed in the vendoring commit where trivial; `#[allow]`ed with a comment where upstream style is deliberate. `prek` must be green.
- [The published `gh-workflow-macros` 0.9.0 differs from the fork's copy] → Checked once at import by diffing the fork's `crates/gh-workflow-macros` against the registry source cargo unpacks. The fork's history shows no change to that crate after upstream's alignment commit, so a difference would be an upstream publish discrepancy, not ours.
- [`gh-workflow`'s `insta` snapshot tests depend on exact `serde_yml` output] → Unchanged dependency versions; the snapshots move only if we change the library, in which case `cargo insta review` is the normal workflow.
- [The `id:` text pass is coupled to serde_yml's field order and indentation] → `id` is the first serialized field of `StepValue` and the match is anchored on indentation and exact id. A mismatch fails generation, never silently drops a suppression. `check-generated` and zizmor in CI both catch a drift that reaches `main`.
- [Deleting the substring check before zizmor runs in CI leaves a window where a cache action in a build job is caught only by a maintainer's hook] → D6 is part of this change, not a follow-up; the tasks order it before the deletion.
- [The upstream fork on GitHub and the in-tree copy drift apart] → The fork stops being a source of truth the day this lands; the README names upstream, not the fork, as what `gh-workflow-sync` fetches.

## Migration Plan

1. Import the crate, point it at the registry macros crate, switch the dependency, add the member, fix lints and formatting, and confirm `check-generated` is unchanged — the YAML must be byte-identical at this step.
2. Add the suppression field and renderer pass; regenerate; the only diff is one comment line per test job in preview and stable.
3. Ensure zizmor runs in CI (D6); remove the `zizmor.yml` ignores; confirm zizmor is clean on the regenerated files.
4. Remove the substring clause from the generator test.
5. Rollback at any step is reverting that step's commit; none changes what a workflow does.

## Open Questions

- Should the local delta be offered upstream? The `FluentBuilder` / `NamedJob` / outputs commit is general-purpose. Does not affect this change.
