## Context

See proposal.md for motivation. The facts that shape the approach:

- `tooling/xtask` already links `cartograph` with its `development` feature and `domain`, and already shells out to `git`, `gh`, and `devenv` through `shell.rs`. It locates workspace packages through `cargo_metadata` in `workspace.rs`. It depends on `heck`.
- The specta exporter (`specta_typescript::Typescript`) can return an export as a `String` (`export`) as well as write it to a path (`export_to`). The current bins use `export_to` and then re-read the file to post-process it.
- `specta_serde::PhasesFormat` emits, for every type whose serialize and deserialize shapes diverge, `X_Serialize`, `X_Deserialize`, and a bare `X = X_Serialize | X_Deserialize`. Those phase names exist only in the exported text; the `Types` collection holds base names.
- `specta-typescript` inlines every referenced type. There is no mode for emitting an import from another module, so cartograph's export always redeclares every domain type it touches.
- Verified empirically (2026-10-08): `patch-types.ts` and `schemagen.ts` produce byte-identical output whether their input is raw specta output or treefmt-formatted output. Both key only on `export type` at column zero, JSDoc fences, and blank lines, all of which raw output has.
- Verified empirically (2026-10-08): the app typechecks identically against the committed (unpatched) cartograph bindings and against fully patched ones. Six pre-existing errors in both runs, all in app code against domain types.
- `just check-generated` already follows a regenerate-then-diff pattern for devenv's managed files: ask xtask for the file list, `git diff HEAD --exit-code` on those paths. The generated `check-generated.yaml` workflow triggers on the sources and outputs it guards and runs `devenv shell -- just check-generated`.
- `treefmt` is on PATH only inside the devenv shell, and its config is tree-rooted, so it formats paths inside the working tree and not scratch copies elsewhere.

## Goals / Non-Goals

**Goals:**

- One `cargo xtask typegen` that is the only way bindings get generated, idempotent against a clean tree.
- A Rust port of the two text-processing scripts whose output is provably identical to theirs, established by the committed tree as golden input.
- The lockstep check covers the bindings with no new mechanism.

**Non-Goals:**

- Replacing the text-level rewrite with reflection over the `Types` collection. The collection is the right source for _which_ types exist, but the exported text is the only place the phase names and the exact emitted declarations exist. A reflection-based patcher would be a second exporter.
- A general "post-process any specta export" framework. Two crates, one direction of dependency.

## Decisions

### D1 (2026-10-08): The pipeline is an xtask task, not a tooling/bin script or a per-crate binary

`cargo xtask typegen` in `tooling/xtask/src/tasks/typegen.rs`, registered as a `Typegen` subcommand with an optional `--only <domain|cartograph>` for iterating on one crate. Internal layout follows `dev_server`: a `typegen/` directory for the export, patch, and schema steps if the file grows past comfortable size; a single file otherwise.

Rejected: keeping the Rust bins where they are and moving only the TypeScript scripts into `tooling/bin` as path-taking CLIs. That leaves the pipeline split across two languages, leaves each crate carrying a `required-features` binary, and gives the lockstep check nothing to call but a chain of recipes. Rejected: a new workspace crate. xtask is the crate for exactly this and already pays the link cost against `cartograph`.

### D2 (2026-10-08): Exports are held in memory; each file is written once

Each crate's export goes through `Typescript::export` into a `String`. Domain's string is scanned for the name set, post-processed, and written. Cartograph's string is post-processed against that name set and written. Nothing is read back from disk mid-pipeline. The header line is set through the exporter's `header` option so each file names `cargo xtask typegen` as its generator.

Rejected: `export_to` followed by read-modify-write, as the bins do today. It works, but it is the reason `patch-types.ts` had to re-read domain's file from disk and the reason the justfile formatted between stages.

### D3 (2026-10-08): The domain name set is scanned from domain's exported text

The set of names cartograph must import is every `export type <name>` at column zero in domain's export string. This is what `patch-types.ts` does and it is why the port is exact: the set includes `RichText`, `RichText_Serialize`, and `RichText_Deserialize` alike, because all three are declarations in the text.

Rejected: iterating domain's `Types` collection for names. It yields `RichText` and not its phase names, so cartograph's `RichText_Serialize` would survive as a local redeclaration. The proposal originally stated this alternative as the plan; it was wrong.

### D4 (2026-10-08): Patch and derive before formatting; format once, from xtask, over the three generated files

The order is export, collapse phase aliases, patch cartograph, derive domain's schema constants, then `treefmt <three paths>` as the last step, run by the task through `std::process::Command` like the existing `git`/`gh`/`devenv` helpers. The empirical check (see Context) licenses dropping the mid-pipeline format; the trailing format is required so that the written files match what the commit holds and the lockstep diff is free of formatting noise.

Rejected: leaving formatting to the `just typegen` recipe. Then `cargo xtask typegen` alone leaves the tree unformatted, and the lockstep recipe would have to know to format too. Rejected: formatting to a scratch path and comparing. treefmt's config is tree-rooted; scratch paths outside the tree do not pick it up.

Consequence: `cargo xtask typegen` requires `treefmt` on PATH, which means the devenv shell. That is already true of the `devenv` and `gh` helpers; the task fails with the same "is it on PATH?" message if it is missing.

### D5 (2026-10-08): The phase-alias collapse keeps its exact current rule

A line of the form `export type X = X_Serialize | X_Deserialize;` becomes `export type X = X_Serialize;`. Applied to both crates' exports from one function. No generalization; the rule is a string equality on the right-hand side, as today.

### D6 (2026-10-08): The cartograph patcher is a line-for-line port

Declaration blocks are found by `export type (\w+)` at column zero; each block extends backward over blank lines into an immediately preceding JSDoc (`/** … */`) and forward to the next block's start, trailing blanks trimmed. Blocks whose name is in the domain set are dropped and their names collected into one sorted `import type { … } from "@waypoints/domain"`. Everything else is kept in order. The regex-free line walk from `patch-types.ts` ports directly; no `regex` crate is added.

### D7 (2026-10-08): Schema constants use `heck::ToPascalCase`

`schemagen.ts` uses `scule.pascalCase`. For the twenty-seven literals in play today (`heading-1`, `list-item`, `enum-item`, …) both produce the same output. Equivalence is proven by the golden diff, not assumed; see Risks.

### D8 (2026-10-08): Lockstep by regenerate-and-diff in `just check-generated`, not a `--check` flag

The recipe runs `cargo xtask typegen` and adds the three generated paths to the list it diffs against `HEAD` (or `jj diff -r @` in the non-colocated branch it already has). The `check_generated` workflow's `SOURCES` gains `crates/domain/**` and `crates/cartograph/**`, and its `OUTPUTS` the two `typescript/` directories, so the job runs when either the types or the bindings move.

Rejected: a `--check` flag that regenerates to memory and compares. It would need to format in memory, which treefmt does not offer, and it duplicates a comparison git already does for the other generated files.

### D9 (2026-10-08): Flavor features are not touched

`development`, `nightly`, `preview`, and `stable` remain declared on every crate that declares them. The only edit is deleting the `[[bin]]` table whose `required-features` pointed at one. This is a maintainer decision: the features exist for feature cascades and compile-context toggling and are not owned by the binaries.

### D10 (2026-10-09): The patcher stays; neither a different exporter nor a single package replaces it

The text-level import rewrite exists because `specta-typescript` inlines every referenced type and offers no way to reference one from another module. Two ways of not needing the rewrite were weighed and rejected.

Rejected: switching the exporter to `ts-rs`. ts-rs never inlines, so the duplication would not arise, but what it emits instead is one file per type with relative-path imports computed between `export_to` locations (`import type { Base36ID } from "../../domain/typescript/Base36ID"`). It has no attribute or feature that maps a dependency crate to a package specifier, so `@waypoints/domain` would never appear in its output (checked against ts-rs 12.0.1 docs and its issue tracker). The deeper mismatch is that ts-rs has one shape per type: it ignores `skip_deserializing` and honours `skip_serializing_if` only beside `#[serde(default)]`, so there is no equivalent of `specta_serde::PhasesFormat` and the `X_Serialize`/`X_Deserialize` split the bindings rely on (see the spec's "The bare type name is the shape the frontend receives"). Adopting it would mean re-annotating every exported type in `domain` and `cartograph`, dropping the vendored `specta-serde` fix for internally-tagged types under phases, and replacing one `types.g.ts` per package with roughly a hundred files. A rewrite of the export story to delete a line walk the golden diff already pins.

Rejected: one `Types` collection exported to one file in one package. With a single export nothing is ever duplicated and the patcher has no job. The cost is the package layout: `@waypoints/domain` carries hand-written `checks.ts`, `defaults.ts`, and `index.ts` beside its generated file, `@waypoints/cartograph` is generated only, and the app imports from them separately (60 files from domain, 19 from cartograph, 13 from both, as of 2026-10-09). Merging is a package reshuffle and an import rewrite across those files, which is a separate decision from where the generator lives. If that layout is ever adopted for its own reasons, the patcher is the first thing it deletes, which is an argument for keeping the patcher small and tested rather than for avoiding it now.

## Risks / Trade-offs

- [The Rust port drifts from the TypeScript scripts' output] → The committed tree is the golden input. After the port, `cargo xtask typegen` must leave domain's two files unchanged apart from the header line, and must change cartograph's file by exactly the import collapse (37 declarations removed, one import added, header changed). Any other hunk is a port bug. This is a task, not a hope.
- [`check-generated` runs more often once it watches `crates/domain/**` and `crates/cartograph/**`] → Acceptable; the job already builds xtask against cartograph for `workflows --check`, so the typegen run adds seconds, not a compile.
- [`treefmt` missing outside the devenv shell] → Same failure mode and message as the `devenv` helper; the recipe and the workflow both run inside the shell.
- [`pnpm-lock.yaml` changes when `scule` leaves `@waypoints/domain`] → The nix frontend derivation's dependency hash must be refreshed, per the comment in `nix/packages/waypoints/_frontend.nix`.
- [`transload-pipeline` references `crates/domain/bin/typegen.rs`] → Out of scope; the maintainer reconciles that change before applying it.
- [Header text changes on domain's committed files] → One-line diff per file, intended, lands with the regeneration commit.

## Migration Plan

Two commits, both on one branch:

1. `build(dev): …` adds the xtask task, removes the four scripts and their manifest entries, rewires the justfiles and the lockstep recipe, regenerates `check-generated.yaml`, and regenerates the three binding files. The golden comparison in Risks is performed before this commit is made.
2. `build(deps): …` refreshes `pnpm-lock.yaml` and the nix frontend hash.

Rollback is reverting the first commit; the old scripts return with it and the committed bindings return to their prior (unpatched) state.

## Open Questions

- Whether `schema.g.ts` should eventually be derived from the `Types` collection's enum variants rather than from the exported text. The text approach is kept here because it is what exists and is proven; the collection approach would be more robust to formatting changes in specta's output. Deferrable: it changes no spec and no task in this change.
