## Context

See proposal.md — Why.
The constraints that shape the approach:

- `bedrock`'s derives emit `::bedrock::…` paths (`crates/bedrock-derive/src/document.rs`), and `bedrock/src/lib.rs` re-exports the whole `bedrock-core` vocabulary so those paths resolve.
  `domain` uses the façade in eight files, all of them derives (`Collection`, `Document`, `Scalar`) or hand impls of core traits (`Field` for `TerrainKind`, the `touch` hook's `WriteOp`).
  No `domain` source names the engine.
- `transload` reaches the engine in `export.rs` and `ingest.rs` (`bedrock::Database`), in `refusal.rs` (`IngestError::Store(bedrock::BedrockError)`), and in `schema.rs`, whose two `jsonschema` calls validate readings on behalf of export and ingest.
  Ingest also mints `TerrainID`s, which is fine: the pure build of `domain` still mints.
- Cargo ignores `default-features = false` on a member's dependency line when the workspace declaration does not set it, with a warning.
  `domain` declares no default features, so only the `bedrock` declaration needs the flag.
- `domain::now_ms` calls `Utc::now`, which needs chrono's `now` feature.
  `domain` does not enable it; today it arrives through other crates' feature unification (`specta`'s chrono support among them).
  With the engine out of the graph the feature may or may not still arrive, depending on who else is being built, which is exactly the kind of dependence a pure build must not have.
- `wasm32-wasip1` has a complete standard library: system randomness, the clock, and standard error work through WASI imports.
- `wasm32-unknown-unknown` has no host at all.
  The predictions on record, from two independent reviews, are: `getrandom` ≥ 0.3 refuses to compile for it without a `getrandom_backend` cfg, and its `wasm_js` backend imports from a JavaScript host; `SystemTime::now` compiles but is unsupported at run time; chrono's `clock` feature pulls `iana-time-zone`, which on that target depends on `js-sys` and `wasm-bindgen`.
  The paths that would carry those crates into a grammar-only graph are known from `cargo tree`: `rand` (direct from `domain`), `uuid` with the workspace's `v4` feature (`domain`, `bedrock-core`, `specta`), and `ahash` under `jsonschema` (gone once `jsonschema` is behind `store`).
  None of this has been built; it is what this change finds out.
- No wasm target is installed in the pinned toolchain today; adding one rebuilds the devenv toolchain.
- House rules: every task lands hook-green; the per-commit hook runs `cargo nextest run --workspace` with no feature flags, so a feature-gated regression can only be caught by an explicit `--no-default-features` check.

## Goals / Non-Goals

**Goals:**

- `cargo check -p domain --no-default-features --features syntax --target wasm32-wasip1` and `cargo check -p transload --no-default-features --target wasm32-wasip1` both succeed, and neither graph contains `turso_core`.
- `cargo build --workspace` and `cargo nextest run --workspace` are unchanged in outcome, and the `store` round-trip over `getting-started` passes.
- Every crate that reaches the engine says so in its own manifest.
- The `wasm32-unknown-unknown` question is answered with a build log, not a prediction: does the grammar compile as is, what does it need if not, and what does a module built from it import.

**Non-Goals (design-level, beyond the proposal's):**

- No public-API stability promise for the pure build.
  Which items `transload` exposes without `store` is whatever the grammar modules export today; a later change may rearrange it.
- No attempt to make `store`, `cartograph`, or `outpost-*` buildable without the engine.
  They open databases; that is their job.
- No fix for whatever the `unknown-unknown` experiment finds in `domain`.
  Findings are recorded; the decision about them belongs to the plugin change and to a hardening change.

## Decisions

### D1 — The pure mode is the façade's, not `domain`'s (2026-10-30)

`bedrock` gains a default-on `engine` feature.
Off, the crate is the `bedrock-core` re-exports plus the derives — exactly the surface the derives' `::bedrock::…` paths and `domain`'s hand impls need.
`domain` depends on the façade with no features and changes no source.
Every crate that opens a database enables `bedrock/engine` through its own edge.
A grammar-only graph — `transload` (no `store`) → `domain` → `bedrock` (no `engine`) → `bedrock-core` — never names the engine.

- **Rejected — `cfg_attr`-gating the derives in `domain`:** eight files of conditional attributes (and the `#[bedrock(id)]` helpers, which only exist when the derive is applied) to express "no engine", when the engine was never `domain`'s dependency to begin with.
  The façade's own docstring says the engine lives behind its boundary; this makes that true at the feature level.
- **Rejected — a `domain-core` split:** the persistence impls are derives on the type definitions; the orphan rule pins them to the crate that owns the types.
- **Rejected — compiling the engine for WebAssembly:** even where it links, an embedded database inside a module that must not open one is the wrong shape, and the size alone defeats the purpose.

### D2 — Features are named at the member, not inherited (2026-10-30)

The workspace declaration for `bedrock` sets `default-features = false`; `store`, `outpost-hold`, and `transload` enable `engine` explicitly, and `domain` enables nothing.
This is the only way cargo allows a member to opt out of a workspace dependency's defaults, and it has a benefit of its own: the manifests now state which crates reach the engine, instead of every crate reaching it by inheritance.
`domain`'s workspace declaration is left alone because `domain` has no default features to turn off.

- **Rejected — leaving the workspace line alone and setting `default-features = false` in `domain`:** cargo ignores it and warns; the engine stays in the graph.
- **Rejected — a second façade crate without the engine:** two names for one vocabulary, and the derives' emitted paths can only name one of them.

### D3 — `transload`'s seam is `store`, and it covers validation (2026-10-30)

`store` (default on) gates `export`, `ingest`, `IngestError::Store`, the reading-validation functions in `schema` (the `jsonschema` call sites), and the `bedrock` and `jsonschema` dependencies, which become optional.
The `_schema.json` face — building the Draft-07 document from a Chart and reading the canonical Chart back out of `x-waypoints` — stays in the pure build; only validating instances against it needs the validator.
The refusal _types_ stay unconditional, since they are part of the grammar's vocabulary; only the variant that wraps an engine error is gated.

- **Rejected — gating `jsonschema` separately from `store`:** nothing in the pure build needs it, and nothing in the default build can do without it; one seam is enough.
- **Rejected — gating `refusal` wholesale:** the malformed-Bundle kinds describe grammar failures and are useful to a grammar-only consumer.

### D4 — `domain` declares the clock it uses (2026-10-30)

`domain` enables chrono's `now` feature itself.
Depending on unification to supply a feature your own code needs is a latent build break that surfaces only when the graph shrinks, which is precisely what this change does.

- **Rejected — gating `now_ms` and the timestamp defaults behind a feature:** that is sandbox hardening, and it reaches into `domain`'s source; this change deliberately changes none.

### D5 — Two targets: `wasip1` is the proof, `unknown-unknown` is the experiment (2026-10-30)

The change is proven by compiling for a WebAssembly target because that is the consumer the pure build exists for and because no host target can show that the engine is truly gone — on a host, the engine compiles whether or not anything needs it.
`wasm32-wasip1` is the required target because its standard library is complete, so it isolates the question this change must answer — is the engine gone — from every other question about the data model.

`wasm32-unknown-unknown` is built too, as an experiment with a recorded outcome, because the plugin contract's sandbox story depends on the answer and the answer is currently a prediction.
The experiment has three steps, each recorded whether it passes or fails:

1. `cargo check -p domain --no-default-features --features syntax --target wasm32-unknown-unknown` and the same for `transload`, as is.
   If both compile, the prediction that `getrandom` blocks the target was wrong, and the sandbox can be a property of the build with no data-model change.
2. If either fails, the same checks with `--cfg getrandom_backend="wasm_js"` for that target in `.cargo/config.toml` (the configuration `getrandom`'s own documentation prescribes for the target), recording which dependency needed it.
   Compiling under that cfg means the target is reachable with a configuration line and no source change, at the cost of host imports to inspect.
3. A throwaway probe crate, not committed — a `cdylib` depending on `transload` without `store` and exporting one function that serializes a `Content` value — built for `unknown-unknown` under whatever configuration step 1 or 2 found, with its import section listed by `wasm-tools`.
   An empty import section means the grammar is a sealed sandbox on that target as it stands; a non-empty one names exactly what the hardening change would have to remove, and from which crate.

The outcome is written into this document's **Outcome** section when the tasks run.
It is an input to the plugin change's choice of target and to any hardening change; it changes nothing here.

- **Rejected — requiring `unknown-unknown`:** if the prediction is right, the requirement forces data-model gating into a change whose whole point is to change no data-model source.
- **Rejected — skipping `unknown-unknown` and assuming the prediction:** the prediction may be wrong, and the cost of finding out is one toolchain target and three commands.
- **Rejected — proving the split on the host only:** `cargo tree -i turso_core` on a host graph shows the dependency gone, but not that what remains compiles for a target without an engine.

## Outcome

_To be filled by the tasks that run the experiment (design D5): for each step, the command, whether it compiled, the first failing dependency if not, and for the probe module the import section verbatim._

## Risks / Trade-offs

- [A `bedrock` item outside the listed modules references the engine] → `cargo check -p bedrock --no-default-features` is the first task's gate; anything it names moves behind the feature.
- [A transitive `domain` or `transload` dependency does not build for `wasm32-wasip1`] → found by the two target checks before anything else lands; the fallback is to gate the dependency behind `store` if `transload`'s, or to record it if `domain`'s, since `domain`'s manifest is the only thing this change may touch there.
- [The `unknown-unknown` experiment is read as a requirement and someone starts gating `domain` to make it pass] → the tasks say "record", not "fix", and the Outcome section is where it ends.
- [Feature unification hides a gating mistake under `nextest --workspace`] → every verification step runs with `-p` and `--no-default-features`, where unification cannot reach.
- [`cargo deny` surfaces advisories or duplicates on the new targets] → triaged here with the file's existing `ignore`-with-reason discipline.
- [The `crash-victim` binary or the Android allocator is forgotten] → both are named in the task; `cargo check -p bedrock --no-default-features` fails if the binary is not gated.

## Migration Plan

1. The toolchain targets land first, alone, so the checks they enable can be run against every later commit.
2. `bedrock/engine` with the workspace declaration and the three member lines, as one commit: the workspace line and the member lines cannot land separately without a warning-only broken state in between.
3. `domain`'s chrono line, then `transload/store`, each with its `wasip1` check.
4. The experiment, as the last task, with its findings committed into this document.
5. Rollback: every feature defaults to today's behavior, and each commit reverts cleanly on its own.

## Open Questions

- Whether `cargo deny` needs the WebAssembly targets in its graph at all, or whether the sync task should learn to exclude verification-only targets.
  Deferrable; if the targets surface nothing, the question is moot.
