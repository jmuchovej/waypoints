## Context

See proposal.md — Why. The facts that shape the approach, read from the tree on 2026-10-10 unless dated otherwise:

- **The fork.** `waypoints-hq/gh-workflow` at `e4f0289` is upstream `tailcallhq/gh-workflow` plus one commit: `fluent.rs` and `outputs.rs` added, `job.rs` and `lib.rs` touched, 186 insertions, no deletions — all in `crates/gh-workflow`. The checkout has no `upstream` remote configured. Three crates: `gh-workflow`, `gh-workflow-macros` (proc-macro, `syn = 3`, untouched by the fork since upstream's version-alignment commit), `gh-workflow-tailcall` (unused here).
- **`gh-workflow-macros` 0.9.0 is published.** On crates.io since 2026-07-22, depending on `heck ^0.5`, `quote ^1`, `syn ^3`. The fork's `gh-workflow` already names it as `{ path = ..., version = "0.9.0" }`, so the registry crate satisfies the same requirement.
- **How it is consumed.** `Cargo.toml` names it as `{ git = ..., rev = "e4f02895aa72", version = "0.9.0" }` in `workspace.dependencies`; `tooling/xtask` is the only consumer.
- **`serde_yml` is deprecated and advisory-bearing** (2026-10-18). The published 0.0.13 describes itself as a thin compatibility shim forwarding every call to `noyalib`, and pins `noyalib = "^0.0.5"`. That range carries RUSTSEC-2026-0333, fixed in `noyalib` 0.0.53; `cargo update -p noyalib --precise 0.0.58` is refused outright, because `0.0.5` and `0.0.58` are separate semver islands and the requirement lives in `serde_yml`'s manifest. `noyalib` itself is maintained and `#![forbid(unsafe_code)]`, latest 0.0.58. The maintained forks of dtolnay's archived `serde_yaml` — `serde_norway` 0.9.42, `serde_yaml_ng` 0.10.0 — are the other candidates. None of the four emits comments from serde data.
- **The generator uses four serializer symbols.** `Workflow::to_string` (`to_string`), `Workflow::parse` (`from_str`), `Error::Yaml(_::Error)`, and `_::from_str::<Value>` twice in `tests/test-workflow.rs`. Every candidate backend spells all four identically.
- **The vendor precedent.** `vendor/specta-macros` and `vendor/specta-serde` are flat copies wired through `[patch.crates-io]`, each with a README naming the upstream version and the patch. They are not workspace members. `.github/renovate.json5` ignores `**/vendor/**`.
- **Workspace conventions that bite.** `derive_setters`, `merge`, and `insta` are not declared at the root, nor is any YAML backend. The fork also pins `async-trait` and `strum_macros`, which `crates/gh-workflow`'s source does not use — the compiler confirms it — so they are not brought to the root. `derive_more` is at the root without features; the crate needs `from`, and relying on another member's feature selection to supply it builds under `--workspace` but fails `cargo test -p gh-workflow` with a `compile_error!`, so the feature is declared on the member. `syn` is `2` at the root for `bedrock-derive`; `syn` 3.0.6 already coexists in `Cargo.lock` as the macros crate's private dependency. `workspace.lints.clippy` differs from the fork's (`use_self = warn`) and sets `disallowed_macros = "deny"`. `treefmt` formats every `*.rs` in the tree; `prek` runs clippy, cargo-check, cargo-test, treefmt.
- **The imported crate carries dead code.** `generate.rs` — `Generate`, `check_file`, `generate`, reached only through `Workflow::generate` — is never called here: xtask has its own writer and its own banner (`# Generated from xtask::workflows::check_generated`). It holds the crate's only three `println!` calls, which `disallowed_macros = "deny"` rejects, and it is the sole user of `Error::OutdatedWorkflow` and `Error::MissingWorkflowFile`. The two `insta` snapshots under `src/snapshots/` are orphaned — the fork deleted the tests that produced them, and nothing in the crate references `insta`.
- **Where third-party code lives.** `vendor/` holds patched application dependencies (the two specta crates). `tooling/` holds development tooling (`xtask`, `icongen`, `bin/`).
- **jj does not support git submodules.** A colocated checkout would leave the submodule path empty under jj, and `cargo` would fail to find the crate. That rules the submodule out regardless of other merits.
- **The renderer already post-processes text.** `WorkflowSource::render` in `workflows.rs` calls the serializer's `to_string`, prepends the family's disclaimer, and trims trailing whitespace line by line.
- **zizmor's inline grammar.** `# zizmor: ignore[rule]`, optionally followed by free text, as a YAML comment on any line within the finding's span. The `cache-poisoning` audit's primary location is the `uses:` line, and it adds the whole step body as a hidden location, so a comment on the step's `id:` line is inside the span. Comments inside strings or block scalars are not recognised.
- **Where zizmor runs.** The `zizmor` hook in `.config/hk.pkl`, locally. No workflow runs it today.

## Goals / Non-Goals

**Goals:**

- A library edit is one commit here, and the library is held to the same hooks as everything else.
- The generator's dependency graph is the repository's to advance; no advisory is carried because a manifest is out of reach.
- A step owns its suppressions; the rendered YAML carries them where zizmor reads them; nothing at file scope.
- zizmor becomes the authority on third-party action risk, in CI, and the generator's tests stop imitating it.

**Non-Goals:**

- Those in the proposal.
- Preserving upstream git history inside this repository.
- Making the renderer a general comment-capable serializer.
- Controlling the serializer's quoting style.

## Decisions

### D1 — One crate in `tooling/`, as a workspace member, synced by a recipe; the macros crate from crates.io

**Chosen** (2026-10-10). `tooling/gh-workflow/` is copied from the fork's `crates/gh-workflow` at `e4f0289` (source, manifest, snapshot fixtures). `Cargo.toml` replaces the git dependency with `gh-workflow = { path = "tooling/gh-workflow" }`, adds the crate to `members`, and adds `gh-workflow-macros = "0.9.0"` to `workspace.dependencies`. A `tooling/gh-workflow/README.md` records upstream repository, upstream revision, the local delta (today: the `FluentBuilder` / `NamedJob` / outputs commit; after this change: the suppression field and the serializer swap as well), and the macros version the source was written against. `just gh-workflow-sync <rev>` fetches `tailcallhq/gh-workflow` at `<rev>` into a scratch directory and copies `crates/gh-workflow` over the in-tree one; the maintainer then uses `jj diff` and `jj resolve` to re-apply the delta and updates the README's revision.

**Why the macros crate is not brought in.** The fork never touched it; the published 0.9.0 is what the fork's `gh-workflow` already declares a dependency on by version; and leaving it in the registry keeps its `syn` 3 a private proc-macro dependency, which is how the lockfile resolves today. Bringing it in-tree would have forced a choice between a workspace-wide `syn` bump that `bedrock-derive` has to survive and a second `syn` key in the workspace table — a cost with nothing on the other side. (The crate's source needs nothing from `syn` 3: it uses `parse_macro_input!`, `Data`, `DeriveInput`, `Fields`, and `syn::Ident`, all present in `syn` 2. That is not a reason to vendor it, only confirmation that the registry copy is unremarkable.)

**Why `tooling/` rather than `vendor/`.** `vendor/` is for patched application dependencies — code that ships in the product's dependency graph and is waiting on an upstream fix. gh-workflow is development tooling: it exists to generate CI, sits beside `xtask`, and is edited as part of the generator. It is third-party in origin and first-party in use, and the directory should follow the use. (An earlier draft chose `vendor/` for its README-and-Renovate-ignore convention; the README convention carries over, and the Renovate point is moot once the crates are workspace members managed through the workspace table.)

**Why a workspace member, like `xtask`.** gh-workflow is live — this change itself extends it — so it is compiled, linted, tested, and formatted by `prek` like the generator that uses it, and it inherits its dependencies from `workspace.dependencies` like every other member. The one-time cost is a reformat under this repository's `rustfmt` settings and whatever clippy raises under its lints; both are absorbed in the import commit.

**Alternatives rejected.**

- _Git submodule._ Unusable under jj (see Context). Would otherwise be the conventional answer.
- _`git subtree add --prefix tooling/gh-workflow`._ Keeps upstream history mergeable with `git subtree pull`, and works in a colocated repo because the result is ordinary commits. Rejected because a subtree is the whole upstream tree — `gh-workflow-tailcall`, upstream's `.github`, Renovate config, release tooling — and every pull re-adds what was pruned. For a 186-line delta, merge history buys little over a diff.
- _Pristine upstream plus a patch series (`patches/*.patch`, applied by a recipe)._ Keeps upstream untouched and the delta explicit. Rejected because it is the opposite of "straightforward to edit": every change is made to the source, then exported to a patch, then re-applied to verify. This is the form a Nix derivation would want, and nothing here is built by Nix.
- _Keep the git dependency and the fork checkout._ The status quo being replaced. No longer merely inconvenient: it is what makes RUSTSEC-2026-0333 unfixable (D7).

### D2 — Every dependency is inherited from the workspace

The manifest uses `workspace = true` for every dependency, including `gh-workflow-macros`, and `[lints] workspace = true`, exactly as `xtask` does. `workspace.dependencies` gains `gh-workflow-macros = "0.9.0"` and the ones the fork pins that the root lacks and the source actually uses — `derive_setters`, `merge`, `insta`, and the YAML backend from D7 — at the fork's versions where they carry over, which are already what `Cargo.lock` resolves. No existing workspace version moves.

Two departures from a plain `workspace = true` sweep, both forced by facts in Context: `async-trait` and `strum_macros` are fork pins the source does not use, so they are dropped rather than added to the root; and `derive_more` is taken as `{ workspace = true, features = ["from"] }`, because the root declares it featureless and a bare inheritance only compiles while some other member happens to enable `from`.

The one coupling to watch: the in-tree `gh-workflow` source is written against a specific macros API. Within 0.9.x that is stable by semver; a Renovate bump to 0.10 lands as an ordinary pull request whose `cargo check` says whether the source still compiles. The README names the macros version the source was taken alongside, so the pairing is not tribal knowledge.

_Alternative rejected:_ crate-local pins for the mismatched dependencies. Makes this the only member whose versions are not the workspace's, which is the kind of exception Renovate and a reader both have to learn about.

### D3 — The suppression travels on the step, skipped by serde, placed by the renderer

`StepValue` gains `#[serde(skip)] pub lint_ignores: Vec<LintIgnore>` where `LintIgnore { rule: String, reason: String }`, and `Step<A>` gains `.zizmor_ignore(rule, reason)`. Before serializing, `render` walks `workflow.jobs` and collects `(job name, step id, Vec<LintIgnore>)` from the typed structure. After serializing and prefixing the disclaimer, a pass finds, within each job's block, the line `- id: <id>` or `id: <id>` at step indentation and appends ` # zizmor: ignore[rule] reason` (several rules comma-joined in one comment). A collected directive with no id, or whose id line is not found exactly once, returns an error naming the job and step; `write` never runs.

**Why the `id:` line.** It is the one field every step we care about can be given, it is unique within a job, it serializes first in `StepValue` (so it is the step's opening `- ` line), and it is inside the finding's hidden span. `uses:` would be the primary span but is absent on `run:` steps; `name:` is free text.

**Why skip rather than a side table.** The directive belongs to the step that needs it; `rust_cache()` declares it where its doc comment already explains it. A side table in xtask keyed by id would separate the reason from the step and go stale on a rename without the compiler noticing. The field is a fork change, but the fork is now in-tree, which is the point.

**Alternatives rejected.**

- _Encode the directive in `name:` and rewrite the quoted scalar._ `#` forces the serializer to quote the string; moving the comment outside the quotes is a rewrite of the scalar's quoting, fragile and specific to one serializer's quoting rules — and D7 shows those rules are not even stable across backends.
- _zizmor's config with `file.yaml:line:col` ignores._ Supported, but line numbers in generated files move on every regeneration.
- _Pick a serializer that emits comments and drop the text pass._ There is none among the serde-driven Rust YAML crates, `noyalib` included. The text pass is independent of D7: whichever backend is chosen, the comment is appended after serialization.

### D4 — `rust_cache()` declares the one suppression this repository needs today

`.id("rust-cache").zizmor_ignore("cache-poisoning", "written only from main (save-if), restored by test jobs that publish nothing")`. The reason is the doc comment's, shortened. The build jobs declare one suppression of their own once `r2-release-build-cache` has landed: `setup_mise` with `cache: true` carries `cache-poisoning` with the reason recorded there (mise's install cache under GitHub's default write rules, tools pinned by `mise.lock`). Everything else in a build job is a `run:` step zizmor does not classify as a cache.

### D5 — The substring clause leaves the generator test; the positive assertions stay

`environments_follow_the_vault_credential` keeps: credential and environment come together; provider and credential belong on the step; and — from `r2-release-build-cache` D7 — a step setting `RUSTC_WRAPPER` follows a server start resolving the `ci` profile's `cache` scope, every `cache-restore-*` / `cache-save-*` step resolves that scope itself, no step's `env` names the store credential, and no `ci` resolution is unscoped. The clause rejecting `uses` containing `cache` or `with.cache == true` is deleted with its comment, and with it that change's by-name allowance for `mise-action`'s cache, which exists only to get past the clause. The comment that remains points at zizmor for third-party actions. If this lands before `r2-release-build-cache`, the clause is deleted anyway; the positive assertion arrives with that change.

### D6 — zizmor in CI, taking the `hk` job when it exists

The requirement is that zizmor runs over the generated workflows on every pull request. If the planned `hk check` job has landed in `pull_request.rs` by the time this is implemented, it already covers `.github/workflows/*.yaml` through the `zizmor` hook and nothing more is needed. If not, `pull_request.rs` gains a job running `zizmor .github/workflows` from the `devtools` mise lane, to be folded into the `hk` job when that lands. `.github/zizmor.yml` is deleted once its `cache-poisoning` block is removed and nothing else remains.

### D7 — `noyalib` 0.0.58 replaces `serde_yml`, and the quoting restyle is proven empty rather than avoided

**Chosen** (2026-10-18). The in-tree crate depends on `noyalib = "0.0.58"` and the four call sites in Context are rewritten to it. `RUSTSEC-2026-0333` and its `.config/deny.toml` ignore are deleted in the same commit.

**Why not stay on `serde_yml`.** It is deprecated by its own author, it is a shim with no function but to forward to `noyalib`, and its `^0.0.5` pin is the entire reason the advisory is unreachable. Keeping it would mean importing the crate in order to make its dependencies editable and then not editing them.

**Why `noyalib` over `serde_norway` or `serde_yaml_ng`.** It is what `serde_yml` already forwards to, so the swap is the shim's removal rather than a change of implementation — the parser and emitter under the generator are the same ones running today, at a fixed version. It is maintained and forbids unsafe code, where the `serde_yaml` forks inherit `unsafe-libyaml`.

**The cost, stated plainly.** Every `noyalib` release is `0.0.x`, so every bump is semver-breaking and `"0.0.58"` already behaves as an exact pin whether or not it is spelled as one. Renovate must treat it as a major: a bump is a pull request to read, not to merge on green. This is a worse version story than `serde_norway`'s `0.9.42`, and it is the deliberate trade for not changing emitter implementations in the same commit that moves the crate in-tree. If the pin becomes a maintenance burden, moving to `serde_norway` later is the same four call sites.

**The quoting restyle.** `noyalib` quotes fewer plain scalars than `serde_yml` did. Regenerating produces a diff in all eight workflows — measured at 68 insertions and 68 deletions — of the form `run: "nix profile install nixpkgs#devenv"` → unquoted and `cron: "0 4 * * 1"` → unquoted. Both remain valid plain scalars: `#` is only a comment introducer when preceded by whitespace, and `*` is only an alias indicator at the start of a scalar.

This is accepted once rather than configured away, and it is verified, not assumed, because `check-generated` compares bytes and would otherwise only tell us the bytes moved. The check is a semantic one: normalise each workflow's committed version and its regenerated version through a strict YAML parser to canonical JSON and require the two to be identical. Run against the measured restyle, all eight files normalise identically and every `cron` value still parses as a string. That comparison is the gate in tasks 1.9 and 5.1; the textual diff is then committed as the import's own change, and `check-generated` is byte-exact again from the next commit onward.

**Alternatives rejected.**

- _Keep the advisory ignore and leave the backend alone._ The ignore's reason (the generator never parses untrusted YAML) is honest and would keep cargo-deny green. Rejected because it is a standing exception whose only justification was that the fix was out of reach, and this change is what puts it in reach.
- _`serde_norway` 0.9.42._ Better versioning and the closest thing to a maintained `serde_yaml`. Rejected for this change because it swaps emitter implementations as well as crates, which widens the restyle beyond what has been measured. It remains the obvious destination if `noyalib`'s `0.0.x` churn becomes the bigger problem.
- _Pin the quoting by post-processing the rendered YAML to re-quote scalars._ Freezes the diff at the cost of a second text pass with no reader. The restyle happens once.

## Risks / Trade-offs

- [Reformatting the in-tree source under this repository's rustfmt settings makes future upstream diffs noisier] → One-time, and accepted; `just gh-workflow-sync` is followed by `jj fix`, so upstream's formatting never persists and the diff against the previous in-tree state is what matters.
- [Clippy under the root's lint table raises warnings in the imported code] → Known concretely: `generate.rs`'s three `println!` calls against `disallowed_macros = "deny"`. Since that module is dead here (Context), deleting it with `Workflow::generate` and the two `Error` variants only it uses resolves the lint at its root and needs no `#[allow]`. `prek` must be green.
- [`noyalib`'s `0.0.x` versioning means every bump is a breaking change] → Accepted in D7 and mitigated by convention, not by the resolver: the pin is exact in effect, and Renovate must surface a bump as a major for review. `serde_norway` is the documented exit.
- [The serializer swap silently changes a workflow's meaning, not just its quoting] → Guarded by D7's parse-and-compare gate rather than by byte equality, which the swap necessarily breaks. A value that changed type or content fails the comparison.
- [The quoting restyle masks a real change in the same commit] → The import commit contains only the move, the manifest rewrite, and the backend swap; the suppression field and `rust_cache()` land in the next, so the restyle diff never shares a commit with a behavioural one.
- [The published `gh-workflow-macros` 0.9.0 differs from the fork's copy] → Checked once at import by diffing the fork's `crates/gh-workflow-macros` against the registry source cargo unpacks. The fork's history shows no change to that crate after upstream's alignment commit, so a difference would be an upstream publish discrepancy, not ours.
- [The `id:` text pass is coupled to the serializer's field order and indentation] → `id` is the first serialized field of `StepValue` and the match is anchored on indentation and exact id. A mismatch fails generation, never silently drops a suppression. `check-generated` and zizmor in CI both catch a drift that reaches `main`.
- [Deleting the substring check before zizmor runs in CI leaves a window where a cache action in a build job is caught only by a maintainer's hook] → D6 is part of this change, not a follow-up; the tasks order it before the deletion.
- [The upstream fork on GitHub and the in-tree copy drift apart] → The fork stops being a source of truth the day this lands; the README names upstream, not the fork, as what `gh-workflow-sync` fetches.

## Migration Plan

1. Import the crate, point it at the registry macros crate, switch the dependency, add the member, swap the YAML backend, drop the dead module and the advisory ignore, fix lints and formatting. `check-generated` is expected to report a diff at this step — the quoting restyle — and the gate is D7's parse-and-compare, not byte equality. The restyled YAML is committed here.
2. Add the suppression field and renderer pass; regenerate; the only diff is one comment line per test job in preview and stable, and `check-generated` is byte-exact from here on.
3. Ensure zizmor runs in CI (D6); remove the `zizmor.yml` ignores; confirm zizmor is clean on the regenerated files.
4. Remove the substring clause from the generator test.
5. Rollback at any step is reverting that step's commit; none changes what a workflow does.

## Open Questions

- Should the local delta be offered upstream? The `FluentBuilder` / `NamedJob` / outputs commit is general-purpose. Does not affect this change.
- Does `noyalib` stay, or is `serde_norway` the destination once the import has settled? D7 takes `noyalib` to keep the emitter fixed across the move; the question is whether `0.0.x` churn costs more than a second restyle would. Revisit after one Renovate bump.
