# gh-workflow

A type-safe GitHub Actions workflow generator, vendored from
[`tailcallhq/gh-workflow`](https://github.com/tailcallhq/gh-workflow). `cargo xtask workflows`
builds every file under `.github/workflows` with it.

## Provenance

| What                 | Value                                                                          |
| -------------------- | ------------------------------------------------------------------------------ |
| Upstream             | `tailcallhq/gh-workflow`, `crates/gh-workflow`                                 |
| Upstream base        | `3793c170f1ead17fff767ad2369fe63802398117`                                     |
| Source taken from    | the fork's `e4f0289`: that base plus the delta, less lint suppressions         |
| `gh-workflow-macros` | 0.9.0 from crates.io (it keeps `syn` 3 privately; the workspace is on `syn` 2) |

`waypoints-hq/gh-workflow` is an archive. Nothing reads it any more; upstream is what
`just gh-workflow-sync <rev>` fetches. Pass a full SHA or a ref, since GitHub will not resolve an
abbreviation over fetch.

## Local delta

Against upstream's `crates/gh-workflow` at the base above:

- **`FluentBuilder`, `NamedJob` and job outputs.**
  New `src/fluent.rs` and `src/outputs.rs`; `src/lib.rs` declares and re-exports them; `NamedJob`
  (a job plus the id it is registered under) lives in `src/job.rs`.
- **`noyalib` replaces `serde_yml`.**
  `src/error.rs` (`Error::Yaml` wraps `noyalib::Error`), `src/workflow.rs` (`to_string` and `parse`)
  and `tests/test-workflow.rs`. This clears RUSTSEC-2026-0333 against the `serde_yml` chain.
- **Dead modules removed.**
  `src/generate.rs` and `src/snapshots/` are gone, along with `Workflow::generate` and the
  `OutdatedWorkflow` / `MissingWorkflowFile` errors that only it used. xtask has its own renderer.
- **`Cargo.toml` rewritten.**
  Every dependency is inherited from the workspace; `derive_more` takes `features = ["from"]`; the
  `insta`, `async-trait` and `strum` dependencies upstream declares are unused here and omitted.
- **No `merge` crate.**
  `Event`, `StepValue` and `Input` merge through inherent `merge` methods instead of
  `#[derive(Merge)]`, which pulled in `proc-macro-error2` (RUSTSEC-2026-0173). The
  `fill_none!` macro in `src/lib.rs` destructures exhaustively, so a new field fails to compile
  until it is listed. A sync brings the derives back; strip them again.
- **Lint suppressions.**
  `Step::zizmor_ignore(rule, reason)` records a `LintIgnore` on `StepValue::lint_ignores`, a
  `#[serde(skip)]` field, so a suppression never reaches the serialized step. `Jobs::iter` exposes
  the jobs in declaration order. xtask's renderer reads both and appends the directive to the step's
  `id:` line. These came after the fork import; they are not in `e4f0289`.
- **Formatting.**
  The workspace's rustfmt style differs from upstream's, so a fresh sync shows reformatting noise
  until the crate is run through `treefmt`.

## Syncing

```sh
just gh-workflow-sync <full-sha-or-ref>
```

This overwrites the crate (keeping this README), so afterwards re-apply the delta above, update the
base revision in the table, and format. Expect the resulting diff to be exactly the list above.

## Dependency hazards

Renovate review-gates both of these on the dependency dashboard (`.github/renovate.json5`) rather
than merging on green.

- **`noyalib` is `0.0.x`.** Cargo treats every `0.0.x` release as incompatible with the last, so
  each bump is breaking and can change how scalars are quoted. A serializer change shows up as a
  restyle of `.github/workflows`; judge it by normalising the committed and regenerated files to
  JSON (`yq -p yaml -o json`), not by bytes. The exit, should `noyalib` stall, is `serde_norway`.
- **`gh-workflow-macros` is a registry crate.** A release past 0.9.x may expect the in-tree source
  to move with it, so a bump here means checking this crate builds against it.
