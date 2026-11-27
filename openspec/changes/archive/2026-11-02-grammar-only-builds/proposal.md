## Why

Everything that reads or writes the Bundle format drags the database engine along with it.
`domain` depends on the `bedrock` façade, the façade depends on `turso` and `turso_core`, and `transload` depends on both — so a program that only wants the data model and the Bundle grammar links an embedded SQLite engine it never opens.
That is harmless inside the app, which opens the engine anyway, and fatal anywhere else: a WebAssembly module cannot link it, and nothing in the workspace today can be compiled for a WebAssembly target at all.
Importer plugins — the next piece of the interchange story — are WebAssembly modules that emit data-model values through transload's grammar, so the grammar crates have to be buildable without the engine before any plugin contract can be written against them.

The engine is the large obstacle, not the only one.
A plugin never mints an identifier and never reads the clock — it reports the identifiers its source already has, and the core mints when it ingests — yet the data model's constructors, `Default` impls, and builder defaults mint and read the clock, and a crate compiles what its manifest declares, not what a caller reaches.
On `wasm32-unknown-unknown`, the target a sealed sandbox wants, that is fatal in a second way: `getrandom` refuses to compile for it at the crate level, and the documented workaround binds the module to a JavaScript host for exactly the two things a plugin must never do.
This change was first run as an experiment to find out whether that prediction held; it did, precisely, and the findings are recorded in `design.md`.
It therefore also makes minting and the clock unavailable on wasm32 targets: the types stay, with their parsing and serialization; the ways to create a fresh identity or read the time do not exist in a wasm32 build.

The result is that only types and grammar compile to WebAssembly.
`domain`, the Bundle grammar in `transload`, the façade's vocabulary and derives, and pure utilities build for both `wasm32-unknown-unknown` and `wasm32-wasip1` with no engine, no entropy, no clock, and no host of any kind in the graph, while the whole workspace builds and tests exactly as before.

## What Changes

- **`bedrock` gains a default-on `engine` feature.**
  Off, the crate is the `bedrock-core` re-exports plus the `Collection`, `Document`, and `Scalar` derives — the surface the derives' `::bedrock::…` paths resolve against.
  Everything turso-backed (the database, transactions, migrations, durability, query planning, the engine error type, the `SyncMode` re-export, the crash-victim test binary, the Android allocator) sits behind the feature.
- **Members name the features they use.**
  Cargo does not let a member turn a workspace dependency's default features off when the workspace declaration leaves them on, so the workspace declaration for `bedrock` turns them off, and every crate that opens a database (`store`, `outpost-hold`, `transload`) enables `engine` on its own line.
  `domain` takes the façade with no features.
- **`transload` gains a default-on `store` feature** around `export`, `ingest`, the store-error variant of `IngestError`, the Draft-07 reading validation in `schema`, and the `bedrock` and `jsonschema` dependencies.
  The grammar — Bundle layout and manifest, frontmatter, identity comments, sidecars, the `_schema.json` face, link names, refusal types — builds without it.
- **Minting and the clock are unavailable on wasm32.**
  In `domain`, every random constructor (`MarkID::new`, `TerrainID::new`, the sync identities, `Base36ID::new` and its Chart, Feature, and Glyph wrappers), every `Default` that mints, every builder default that mints or reads the clock, the `touch` hook, `now_ms`, and the one `From<String>` that mints an option id are compiled only when the target is not `wasm32`.
  On wasm32 the builders require those fields instead of filling them.
  `domain`'s manifest moves `rand`, uuid's `v4`, and chrono's `now` into a non-wasm32 target table, so a wasm32 build of the data model declares no entropy and no clock.
  The workspace `uuid` drops to `serde`; `domain`, `cartograph`, and `xtask`, the three crates that mint, enable `v4` on their own lines, and the unused `js` feature goes.
- **Both WebAssembly targets join the toolchain**, `wasm32-unknown-unknown` and `wasm32-wasip1`, and `cargo deny`'s graph targets follow by the existing sync task.
  `domain` and `transload` compile for both with no engine, no `getrandom`, no `rand`, and no `wasm-bindgen` in the graph; a probe module built from the grammar for `unknown-unknown` has an empty import section.
- **A Bundle directory regression is fixed on the way.**
  `layout::chart_dir` formatted the Chart id through `Display`, which strips the `builtin.` prefix, so the exporter wrote `library/page/` where the committed getting-started Bundle and three transload tests expect `library/builtin.page/`; it now reaches the inner value through `AsRef`.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `storage`: the storage vocabulary (collection specs, field encoding, the derives) is usable without the engine; a crate that defines collections links no database.
- `data-model`: the data model is usable without a storage engine on a WebAssembly target, and on such a target identifier minting and the clock do not exist — the types parse, serialize, and compare, and nothing creates a fresh identity or reads the time.
- `transload`: the Bundle grammar is usable without the persistence engine and without any source of entropy; export, ingest, and reading validation are separable from the grammar they use.

## Non-goals

- **No behavior change on host targets.**
  Every constructor, default, and hook behaves exactly as before where it exists; the whole workspace builds and tests as before, and the committed `getting-started` Bundle round-trips as before.
- **No new way to mint or stamp.**
  Nothing is added for wasm32 in place of what is removed; a wasm32 build that needs a fresh identifier is a bug by construction.
- **No plugin contract, SDK, runtime, or Fragment grammar.**
  This change makes those buildable and sealed; it does not build them.
- **No relocation of specs.**
  The three deltas land in their existing domains.

## Impact

**Code**

- `crates/bedrock` — `engine` feature (default on) gating `config`, `database`, `ddl`, `durability`, `error`, `explain`, `migrate`, `pool`, `query`, `tx`, `value`, the `SyncMode` re-export, the `turso`, `turso_core`, `tokio`, `tracing`, and `indoc` dependencies, `required-features = ["engine"]` on the `crash-victim` binary, and the Android `mimalloc` dependency.
  The façade re-exports and the derives stay unconditional.
- `Cargo.toml` (workspace) — the `bedrock` declaration gains `default-features = false`; `uuid` drops to `serde`.
- `crates/store`, `crates/outpost-hold`, `crates/transload` — `bedrock` with `features = ["engine"]` (`transload` only under its `store` feature).
- `crates/cartograph`, `tooling/xtask` — `uuid` with `v4`.
- `crates/domain` — `bedrock` with no features; `rand`, `uuid/v4`, and `chrono/now` in a `cfg(not(target_arch = "wasm32"))` target table; `cfg(not(target_arch = "wasm32"))` on the minting and clock sites in `mark/mark.rs`, `chart/ids.rs`, `sync/identity.rs`, `terrain/terrain_id.rs`, and `chart/feature_kind/select_builder.rs`, and `cfg_attr` builder attributes on the id and timestamp fields of `Mark`, `Terrain`, `Landmark`, `Chart`, `Feature`, `SelectOption`, and `SelectGroup`; `now_ms` re-exported only off wasm32.
- `crates/transload` — `store` feature (default on); `bedrock` and `jsonschema` optional behind it; `export`, `ingest`, `IngestError::Store`, and the two `jsonschema` call sites in `schema` gated; `layout::chart_dir` reads the id through `AsRef`.
- `rust-toolchain.toml` — `wasm32-wasip1` and `wasm32-unknown-unknown`; `.cargo/deny.toml` follows by the sync task.

**Dependencies**

- None added.
  A wasm32 build of `domain` loses `rand`, `getrandom`, and `wasm-bindgen`; a grammar-only build of `transload` additionally loses `bedrock`, `turso`, `turso_core`, `tokio`, `tracing`, and `jsonschema`.

**Systems**

- `cargo deny` evaluates the graph for two more targets; anything it surfaces is triaged here.
- CI gains `cargo check` invocations for both WebAssembly targets (crate-level; no workflow rewrite beyond what `check-generated`'s step shape already provides).

**Specs**

- Amends `storage` (new requirement), `data-model` (two new requirements), and `transload` (new requirement).
