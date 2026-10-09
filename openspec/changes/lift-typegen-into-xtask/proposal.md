## Why

The TypeScript bindings the app consumes are produced by four scripts spread across two library crates: a `typegen` binary in each of `domain` and `cartograph` (gated behind `required-features = ["development"]`), a `schemagen.ts` that derives discriminator constants from domain's output, and a `patch-types.ts` that replaces cartograph's inlined copies of domain types with imports. Two of those are Rust, two are TypeScript run under bare `node`, the two Rust ones duplicate the same post-processing function, and `just typegen` orchestrates them through per-crate justfiles with `treefmt` interleaved between stages. Nothing in CI checks that the committed bindings match the Rust types they are generated from.

That gap has already bitten. The committed `crates/cartograph/typescript/types.g.ts` is byte-identical to "specta export, then treefmt, skip `patch-types`": it carries no `import type` from `@waypoints/domain` and redeclares 37 domain types. The app has been typechecking against two structurally equal but nominally distinct sets of type identities, and nobody noticed because the only guard was remembering to run the right `just` recipe. `tooling/xtask` exists precisely for tooling that needs to link app crates without shipping in one, and it already links `cartograph` with its `development` feature and `domain`.

## What Changes

- **One `cargo xtask typegen` replaces the four scripts.** The task registers both specta collections, exports each, collapses the serialize/deserialize phase aliases once (one function instead of two copies), rewrites cartograph's bindings to import the types domain already exports, derives domain's `schema.g.ts` constants, and runs a targeted `treefmt` over the three generated files at the end. The `just typegen` recipe calls it; the per-crate `domain::typegen` and `cartograph::typegen` recipes go.
- **No formatting between stages.** Patching and schema derivation were verified to produce byte-identical output from raw specta output and from formatted output, so the single `treefmt` pass at the end is the only formatting step. The "run treefmt first" precondition in `patch-types.ts` was stale.
- **Both exports are held in memory and written once.** `patch-types.ts` learns which names belong to domain by re-reading domain's generated file from disk. In xtask the exporter returns each binding as a string, so domain's export is scanned in memory for the name set (which must include the `_Serialize`/`_Deserialize` phase names, which exist only in the exported text, not in the type collection), cartograph's export is rewritten against it, and each file is written exactly once. Carving the duplicate declarations out of cartograph's output remains text surgery over specta's emitted form, because `specta-typescript` has no mode for referencing a type from another module.
- **The two library crates stop carrying dev tooling.** `[[bin]] typegen` and `bin/typegen.rs` are removed from `domain` and `cartograph`; `bin/schemagen.ts` and `bin/patch-types.ts` are removed; `scule` is dropped from `@waypoints/domain`'s `package.json`; `specta-serde` moves from both crate manifests to xtask. `specta-typescript` stays a dependency of `domain`, whose library code uses `specta_typescript::Unknown` in field annotations.
- **The flavor features are untouched.** `development`, `nightly`, `preview`, and `stable` stay declared on every crate that declares them today. They exist for feature cascades and compile-context toggling independent of the binaries that once required them; removing the `typegen` binary removes only its `required-features` line.
- **`check-generated` covers the bindings.** The `just check-generated` recipe runs `cargo xtask typegen` and diffs `crates/domain/typescript/*.g.ts` and `crates/cartograph/typescript/types.g.ts` against the commit, the same regenerate-then-diff pattern it already applies to devenv's managed files. The generated workflow's trigger paths gain the two crates' sources and the generated outputs. No `--check` flag on the task: the committed file is the baseline, and git already answers whether the regenerated one matches it.
- **The committed cartograph bindings are regenerated through the full pipeline.** The 37 redeclared domain types collapse to one `import type { … } from "@waypoints/domain"`. The app typechecks identically before and after: the same six pre-existing errors, all in app code against domain types, none involving cartograph.

Nothing here is user-visible. The generated types are the same types with one owner each.

## Capabilities

This change lands in **`toolchain`**. The registry scopes it to how the repository builds, tests, and formats itself, and to the lockstep between generated files and their sources. The TypeScript bindings are a generated file in exactly the sense `tool-projection` and `lockfile-integrity` already use: produced from a single source by a committed generator, committed alongside it, and verified not to drift. `toolchain` is a tree, so this takes a new area beside them.

### New Capabilities

- `toolchain/generated-bindings`: how the TypeScript bindings for the Rust data model are produced and kept honest. One command regenerates every binding from the registered Rust types; a downstream package's bindings import a type the upstream package already exports rather than redeclaring it, so each type has one identity across the frontend; generated bindings pass through the repository formatter and nothing else; and the committed bindings are covered by the generated-file lockstep check.

### Modified Capabilities

None. `toolchain/tool-projection` (pending in `mise-env-configs`) specifies the lockstep check for projected tool configuration; this change adds a new class of file to the same check without changing what the check is. `data-model` governs which types exist and what they mean, not how they are exported.

## Impact

- **`tooling/xtask`**: new `tasks::typegen` module and `Typegen` subcommand; `specta-serde` and `specta-typescript` added to its manifest; `heck` (already a dependency) replaces `scule` for PascalCase. The task locates each crate's `typescript/` directory through the existing `cargo_metadata` workspace loader.
- **`crates/domain`**: `[[bin]]` table, `bin/typegen.rs`, `bin/schemagen.ts`, and the `specta-serde` dependency removed; `scule` removed from `package.json`. `typescript/index.ts` and its hand-written siblings are unchanged.
- **`crates/cartograph`**: `[[bin]]` table, `bin/typegen.rs`, `bin/patch-types.ts`, and the `specta-serde` dependency removed. `typescript/types.g.ts` regenerated with domain imports.
- **`justfile`, `crates/domain/justfile`, `crates/cartograph/justfile`**: root `typegen` calls `cargo xtask typegen`; the per-crate `typegen` recipes removed; `check-generated` gains the typegen regenerate-and-diff.
- **`tooling/xtask/src/tasks/workflows/check_generated.rs`**: trigger paths gain `crates/domain/**` and `crates/cartograph/**`; `.github/workflows/check-generated.yaml` regenerates from that.
- **`pnpm-lock.yaml`**: `scule` leaves `@waypoints/domain`'s importer entry. The nix frontend derivation's dependency hash follows any lockfile change and must be refreshed.
- **Open change overlap**: `transload-pipeline` has a task that edits `crates/domain/bin/typegen.rs` to un-export `RichReference`. After this change that registration lives in xtask's typegen task. The maintainer will reconcile that change's artifacts separately before applying it.
- **Risk**: the Rust port of the two text-processing scripts could drift from their output. The guard is the golden comparison the committed tree already provides: regenerate, and the diff against the commit must be empty for domain's two files and must be exactly the import collapse for cartograph's.

## Non-goals

- Moving `tooling/icongen` or any script in `tooling/bin` into xtask. icongen is a TypeScript image pipeline with its own dependencies and is a different kind of tool.
- Fixing the six pre-existing app typecheck errors. They predate this change and involve domain types, not the generation pipeline.
- Changing which Rust types are registered for export, or how `specta` annotates them.
- Removing or renaming any crate's `development`, `nightly`, `preview`, or `stable` feature.
- Teaching `specta-typescript` to emit cross-module imports. The text-level rewrite stays until upstream offers that.
- A `--check` mode on the task. The lockstep check compares against the commit; a second comparison path inside xtask would duplicate it.
- Reconciling `transload-pipeline`'s reference to the old binary path. That is an update to that change, made by its owner.
