## Why

Everything that reads or writes the Bundle format drags the database engine along with it.
`domain` depends on the `bedrock` façade, the façade depends on `turso` and `turso_core`, and `transload` depends on both — so a program that only wants the data model and the Bundle grammar links an embedded SQLite engine it never opens.
That is harmless inside the app, which opens the engine anyway, and fatal anywhere else: a WebAssembly module cannot link it, and nothing in the workspace today can be compiled for a WebAssembly target at all.
Importer plugins — the next piece of the interchange story — are WebAssembly modules that emit data-model values through transload's grammar, so the grammar crates have to be buildable without the engine before any plugin contract can be written against them.

There is a second question the plugin contract depends on, and it can only be answered by building: how far the grammar crates are from a sealed sandbox.
A `wasm32-wasip1` module gets randomness, a clock, and standard error from the host through WASI imports; a `wasm32-unknown-unknown` module gets nothing from anyone, and whether the grammar crates can be built for it — and what they import if they can — decides whether a plugin's sandbox is a property of its build or a policy of its host.
Two independent reviews predicted that `unknown-unknown` fails on the data model's randomness and clock and would need the data model's constructors gated; the prediction is plausible and unverified, and gating the data model on a prediction is exactly the kind of churn this project avoids.
So this change separates the engine from the vocabulary along a seam the crates already have, proves the separation on the target that is known to work, and runs the experiment on the one that is not, recording what it finds so the plugin change can choose with evidence.

## What Changes

- **`bedrock` gains a default-on `engine` feature.**
  Off, the crate is the `bedrock-core` re-exports plus the `Collection`, `Document`, and `Scalar` derives — the surface the derives' `::bedrock::…` paths resolve against.
  Everything turso-backed (the database, transactions, migrations, durability, query planning, the engine error type, the `SyncMode` re-export, the crash-victim test binary, the Android allocator) sits behind the feature.
- **Members name the features they use.**
  Cargo does not let a member turn a workspace dependency's default features off when the workspace declaration leaves them on, so the workspace declaration for `bedrock` turns them off, and every crate that opens a database (`store`, `outpost-hold`, `transload`) enables `engine` on its own line.
  `domain` takes the façade with no features.
  The app build is unchanged; it is now explicit about which crates reach the engine.
- **`transload` gains a default-on `store` feature** around `export`, `ingest`, the store-error variant of `IngestError`, the Draft-07 reading validation in `schema`, and the `bedrock` and `jsonschema` dependencies.
  The grammar — Bundle layout and manifest, frontmatter, identity comments, sidecars, the `_schema.json` face, link names, refusal types — builds without it.
- **`domain` declares its own clock.**
  `now_ms` calls `Utc::now`, which needs a chrono feature `domain` has so far received only through other crates' feature unification.
  `domain` enables it directly so the pure build does not depend on who else is in the graph.
- **Both WebAssembly targets join the toolchain**, `wasm32-wasip1` and `wasm32-unknown-unknown`, and `cargo deny`'s graph targets follow by the existing sync task.
  Compiling `domain` and `transload` for `wasip1` with no engine in the graph is this change's required proof.
  Compiling them for `unknown-unknown` is this change's experiment: whether it compiles as is, what configuration it needs if not, and what a module built from the grammar imports — recorded in `design.md` as findings, whatever they are.
  The experiment changes no source; if `unknown-unknown` needs the data model gated, this change says so and stops.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `storage`: the storage vocabulary (collection specs, field encoding, the derives) is usable without the engine; a crate that defines collections links no database.
- `data-model`: the data model is usable without a storage engine, including on a WebAssembly target.
- `transload`: the Bundle grammar is usable without the persistence engine; export, ingest, and reading validation are separable from the grammar they use.

## Non-goals

- **No behavior change** in any crate under default features.
  The whole workspace builds and tests as before; the committed `getting-started` Bundle round-trips as before.
- **No change to `domain`'s source.**
  Its manifest changes; its types, derives, constructors, and defaults do not — even if the experiment shows that `unknown-unknown` would need them to.
- **No gating of identifier minting or of the clock.**
  A pure build of `domain` still mints ids and reads the clock through the standard library.
  Removing them is sandbox hardening with its own change, and the experiment's findings are that change's input.
- **No requirement that `wasm32-unknown-unknown` build.**
  It is investigated, not required; the `wasip1` checks are the gate.
- **No plugin contract, SDK, runtime, or Fragment grammar.**
  This change makes those buildable; it does not build them.
- **No relocation of specs.**
  The three deltas land in their existing domains.

## Impact

**Code**

- `crates/bedrock` — `engine` feature (default on) gating `config`, `database`, `ddl`, `durability`, `error`, `explain`, `migrate`, `pool`, `query`, `tx`, `value`, the `SyncMode` re-export, the `turso`, `turso_core`, `tokio`, `tracing`, and `indoc` dependencies, `required-features = ["engine"]` on the `crash-victim` binary, and the Android `mimalloc` dependency.
  The façade re-exports and the derives stay unconditional.
- `Cargo.toml` (workspace) — the `bedrock` declaration gains `default-features = false`.
- `crates/store`, `crates/outpost-hold`, `crates/transload` — `bedrock` with `features = ["engine"]` (`transload` only under its `store` feature).
- `crates/domain` — `bedrock` with no features; `chrono` with the clock feature `now_ms` needs.
- `crates/transload` — `store` feature (default on); `bedrock` and `jsonschema` become optional behind it; `export`, `ingest`, `IngestError::Store`, and the two `jsonschema` call sites in `schema` are gated.
- `rust-toolchain.toml` — `wasm32-wasip1` and `wasm32-unknown-unknown`; `.cargo/deny.toml` follows by the sync task.
- `.cargo/config.toml` — only if the experiment shows `unknown-unknown` needs a target-specific `cfg` to compile, and then only to record what was needed, not to make the target a requirement.

**Dependencies**

- None added.
  A grammar-only build of `transload` loses `bedrock`, `turso`, `turso_core`, `tokio`, `tracing`, and `jsonschema` relative to today.

**Systems**

- `cargo deny` evaluates the graph for two more targets; anything it surfaces is triaged here.
- CI gains two `cargo check` invocations for the `wasip1` target (crate-level; no workflow rewrite beyond what `check-generated`'s step shape already provides).
  The `unknown-unknown` experiment is run by hand and recorded, not wired into CI.

**Specs**

- Amends `storage` (new requirement), `data-model` (new requirement), and `transload` (new requirement).
