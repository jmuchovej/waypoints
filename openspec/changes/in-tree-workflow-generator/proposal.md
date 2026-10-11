## Why

Two frictions in the workflow generator share a root cause: the library it is built on lives in another repository.

The first is editing. `gh-workflow` is consumed as a git dependency on a fork (`waypoints-hq/gh-workflow`, pinned by rev). Our local delta is one commit — `FluentBuilder`, `NamedJob`, and the `{Job,Step}Output` handles — but every further change means a commit in that checkout, a push, a rev bump here, and a `Cargo.lock` move, before the generator can use it. That cost is why the generator carries workarounds that belong in the library.

The second is lint suppression. zizmor's `cache-poisoning` audit flags the test jobs' `rust-cache` in the preview and stable workflows. Those are false positives — the test jobs publish nothing, write the cache only from `main`, and GitHub itself now gives low-trust triggers read-only access to the default-branch cache scope, so the audit's premise is a misconfiguration this repository does not have — but zizmor's inline `# zizmor: ignore[...]` comment is the only suppression that is step-scoped, and `serde_yml` cannot emit comments. So the ignores went into `.github/zizmor.yml` at file granularity, which also silenced the audit for the build jobs in the same files, and a substring check on step names was added to the generator's tests to cover what zizmor no longer did. The result is a hand-rolled, name-based reimplementation of a lint we already run, guarding a trust boundary that the `r2-release-build-cache` change has since written down properly.

Bringing the library in-tree makes step-scoped suppressions a small feature of the step type and a text pass in the renderer, which the renderer already has for its header and whitespace. With that, the file-level ignores and the substring check both go.

## What Changes

- **`gh-workflow` moves into the repository** under `tooling/`, beside `xtask`, as a workspace member that inherits its dependencies from the workspace like every other crate. `tooling/` rather than `vendor/`: `vendor/` holds patched application dependencies, and this is development tooling. The upstream revision it was taken from and the local delta are recorded in a README beside the source; a `just` recipe re-fetches upstream at a named revision so the local changes can be rebased onto it. The git dependency and the separate fork checkout are retired.
- **`gh-workflow-macros` comes from crates.io at 0.9.0.** The fork never modified it, and the published crate matches the fork's. Its `syn` 3 stays a private dependency of a proc-macro crate, as it already is in the lockfile, so the workspace's `syn` 2 and `bedrock-derive` are untouched. `gh-workflow-tailcall` is not brought in; nothing here uses it.
- **Steps carry lint suppressions.** A step gains a `zizmor_ignore(rule, reason)` builder. The directive is not serialized; the renderer collects every `(step id, directive)` pair from the typed workflow before serializing and appends `# zizmor: ignore[rule] reason` to that step's `id:` line afterwards. A directive on a step without an id, or whose id line cannot be found in the rendered text, fails generation.
- **`rust_cache()` carries its own suppression** — `cache-poisoning`, with the reason it has always had: written only from `main`, restored by jobs that publish nothing.
- **`.github/zizmor.yml` loses its `cache-poisoning` ignores.** zizmor then audits every job in every release workflow, including the build jobs it could not see before.
- **The generator's substring cache check is removed.** `environments_follow_the_vault_credential` keeps the environment-and-credential pairing and the positive cache-step assertion added by `r2-release-build-cache`; the "`uses:` contains `cache`" clause goes, because zizmor's curated table now covers those jobs directly.
- **zizmor runs against the generated workflows in CI.** Today it runs only through `hk` locally. The pull-request workflow must execute it — through the planned `hk check` job if that has landed, otherwise as a direct step — so that a suppression that stops matching, or a cache that creeps into a build job, fails the pull request rather than only a maintainer's hook.

Nothing here changes what any workflow does. The YAML differs by a handful of trailing comments.

## Capabilities

This change lands in **`toolchain`**: it governs how the repository produces and checks its own CI definitions, and constrains nothing about the app. It follows `r2-release-build-cache`, which it assumes has landed — the positive cache-step assertion that change adds is what remains of the cache guard once the substring check is removed. `toolchain` is a tree, so this takes a new area.

### New Capabilities

- `toolchain/workflow-generation`: how the workflow generator is sourced and how its output is checked. The generator's library is part of the repository, with upstream provenance recorded; lint suppressions are declared on the step they concern and emitted with it, never at file scope; generated workflows pass the workflow linter with no configuration-file ignores, in CI; and the generator does not re-implement the linter's checks by name.

### Modified Capabilities

None. `toolchain/secret-provisioning`'s "publishing job restores no cache that a run outside that environment could have written" (pending in `secretspec-onepassword`) stays true and becomes zizmor's to enforce on the actions it knows, and the cache-step assertion's for the R2-backed caches.

## Non-goals

- Changing what any generated workflow does, beyond the comment lines.
- Contributing the local delta upstream. Worth doing; separate.
- A general comment-emitting YAML serializer. The text pass handles exactly one directive on exactly one anchor line.
- Bringing in any other git or crates-io dependency, or revisiting the specta patches under `vendor/`.
- Adding the `hk` CI job itself. It is planned independently; this change requires that zizmor run in CI and takes the `hk` job if present.

## Impact

- `tooling/gh-workflow/` — new, copied from the fork's `crates/gh-workflow` at `e4f0289` (upstream `tailcallhq/gh-workflow` plus one local commit), with a manifest that inherits every dependency and the lint table from the workspace.
- `Cargo.toml` — `gh-workflow` becomes a path dependency; the crate joins `members`; `workspace.dependencies` gains `gh-workflow-macros = "0.9.0"`, `serde_yml`, `derive_setters`, `merge`, `strum_macros`, `async-trait`, and `insta`.
- `Cargo.lock` — the git source entry is replaced; `gh-workflow-macros` resolves from the registry instead.
- `.github/renovate.json5` — the new crate is an ordinary workspace member, and `gh-workflow-macros` an ordinary registry dependency; Renovate manages both through the workspace table. A bump of `gh-workflow-macros` past what the in-tree `gh-workflow` was written against is the one thing to watch, noted in the README.
- `justfile` — `gh-workflow-sync <rev>`.
- `tooling/xtask/src/tasks/workflows.rs` — the renderer's directive pass; the test's substring clause removed.
- `tooling/xtask/src/tasks/workflows/_steps.rs` — `rust_cache()` gains an id and a suppression.
- `.github/zizmor.yml` — the `cache-poisoning` block removed; the file is deleted if nothing remains.
- `.github/workflows/pull-request.yaml` — regenerated with a zizmor step if the `hk` job has not landed.
- `.github/workflows/release-{preview,stable}.yaml` — regenerated; one trailing comment per test job.
- `~/Documents/src/github.com/waypoints-hq/gh-workflow` — retired as a working checkout once the in-tree copy is the source of truth; the GitHub fork can stay as an archive.
