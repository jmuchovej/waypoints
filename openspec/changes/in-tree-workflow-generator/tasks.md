## 1. Bring the library in-tree

- [ ] 1.1 Add `gh-workflow-macros = "0.9.0"`, `serde_yml`, `derive_setters`, `merge`, `strum_macros`, `async-trait`, and `insta` to `workspace.dependencies` at the versions `Cargo.lock` already resolves; verify `cargo metadata` shows no version change for any existing dependency and `syn` 2 remains the workspace's
- [ ] 1.2 Confirm the published macros crate matches the fork's: `cargo fetch`, then `diff -r` the fork's `crates/gh-workflow-macros/src` against the unpacked `gh-workflow-macros-0.9.0/src` under `~/.cargo/registry/src`; verify the diff is empty (or record any difference in the README before proceeding)
- [ ] 1.3 Copy `crates/gh-workflow` from the fork at `e4f0289` into `tooling/gh-workflow/` (source, manifest, snapshot fixtures); verify `diff -r` against the fork's crate directory is empty before any edits
- [ ] 1.4 Rewrite the manifest per design.md D2: every dependency `workspace = true` including `gh-workflow-macros`, `[lints] workspace = true`, package metadata trimmed to what a workspace member carries; verify `cargo metadata` resolves each dependency to the workspace version and `gh-workflow-macros` to the registry
- [ ] 1.5 In `Cargo.toml`, replace the git `gh-workflow` entry with `{ path = "tooling/gh-workflow" }` and add the crate to `members`; verify `Cargo.lock` no longer contains the `waypoints-hq/gh-workflow` git source and `cargo check --workspace` passes
- [ ] 1.6 Write `tooling/gh-workflow/README.md` naming upstream (`tailcallhq/gh-workflow`), the upstream revision brought in, the macros version the source was taken alongside, and each local change against upstream; verify it lists the `FluentBuilder` / `NamedJob` / outputs delta with the files it touches
- [ ] 1.7 Add `just gh-workflow-sync <rev>` that fetches upstream at `<rev>` into the scratch directory and copies `crates/gh-workflow` over the in-tree one; verify by running it against the current upstream revision and confirming `jj diff` shows exactly the local delta from 1.6 plus the manifest rewrite from 1.4
- [ ] 1.8 Run `jj fix` and `prek run`; fix or `#[allow]` (with a comment) whatever clippy raises in the imported crate; verify clippy, cargo-test (including the imported crate's `insta` tests), and treefmt are green
- [ ] 1.9 Run `just check-generated`; verify the committed workflows are byte-identical — the import alone must change no YAML

## 2. Step-scoped suppressions

- [ ] 2.1 Add `LintIgnore { rule, reason }` and `#[serde(skip)] lint_ignores: Vec<LintIgnore>` to `StepValue`, with `Step<A>::zizmor_ignore(rule, reason)`; verify a unit test that a step with a directive serializes to the same YAML as one without
- [ ] 2.2 In `WorkflowSource::render`, collect `(job, step id, directives)` from the typed workflow before `to_string`, then append ` # zizmor: ignore[rules] reason` to the matching `id:` line within that job's block after the disclaimer is prefixed; verify unit tests cover: one directive placed; two rules comma-joined; directive on a step without an id fails naming the job; id not found fails; id appearing in two jobs is placed in each correctly
- [ ] 2.3 Give `rust_cache()` `.id("rust-cache")` and the `cache-poisoning` suppression per design.md D4; verify `cargo xtask workflows` changes only `release-preview.yaml` and `release-stable.yaml` (and `pull-request.yaml` / `release-nightly.yaml` only by the added `id:` and comment on their test jobs), one line per test job
- [ ] 2.4 Run `zizmor .github/workflows` with `.github/zizmor.yml` temporarily emptied; verify the suppressions are honoured (no `cache-poisoning` findings on the test jobs) and no other finding appears

## 3. zizmor in CI

- [ ] 3.1 If the `hk check` job has landed in `pull_request.rs`, verify its hook list includes `zizmor` over `.github/workflows/*.yaml` and skip 3.2; otherwise add a `zizmor` job to `pull_request.rs` running `zizmor .github/workflows` from the `devtools` mise lane; verify the regenerated `pull-request.yaml` runs it and a deliberately injected unsuppressed `actions/cache` step in a build job fails the job locally with `act` or by reading zizmor's exit code
- [ ] 3.2 Remove the `cache-poisoning` block from `.github/zizmor.yml`, deleting the file if nothing remains; verify `zizmor .github/workflows` is clean and the `hk` `zizmor` hook passes

## 4. Retire the name-based guard

- [ ] 4.1 Delete the `uses` / `with.cache` clause and its comment from `environments_follow_the_vault_credential`, leaving the credential-and-environment pairing and (once `r2-release-build-cache` has landed) the positive assertions that a `RUSTC_WRAPPER` step follows a server start resolving the `ci` profile's `cache` scope, that every `cache-restore-*` / `cache-save-*` step resolves that scope itself, that no step `env` names the store credential, and that no `ci` resolution is unscoped; update the remaining comment to point at zizmor for third-party actions; verify `cargo nextest run -p xtask` passes and `rg "contains\(\"cache\"\)" tooling/xtask` finds nothing
- [ ] 4.2 Update `.github/zizmor.yml`-referencing comments elsewhere (`_steps.rs`, `workflows.rs`) so none describes the deleted file or the deleted clause; verify `rg "zizmor.yml" tooling .github` finds nothing

## 5. Lockstep and hooks

- [ ] 5.1 Run `just check-generated`; verify the committed workflows match regeneration
- [ ] 5.2 Run `prek run`; verify clippy, cargo-check, cargo-test, and treefmt are green across the change
- [ ] 5.3 Retire the fork as a working checkout: note in `tooling/gh-workflow/README.md` that `waypoints-hq/gh-workflow` is an archive and upstream is what `gh-workflow-sync` fetches; verify the memory file `gh-workflow-local-checkout.md` is updated or removed so it no longer points at the old path
