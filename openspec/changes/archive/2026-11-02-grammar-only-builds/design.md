## Context

See proposal.md — Why.
The constraints that shape the approach:

- `bedrock`'s derives emit `::bedrock::…` paths (`crates/bedrock-derive/src/document.rs`), and `bedrock/src/lib.rs` re-exports the whole `bedrock-core` vocabulary so those paths resolve.
  `domain` uses the façade in eight files, all of them derives (`Collection`, `Document`, `Scalar`) or hand impls of core traits (`Field` for `TerrainKind`, the `touch` hook's `WriteOp`).
  No `domain` source names the engine.
- `transload` reaches the engine in `export.rs` and `ingest.rs` (`bedrock::Database`), in `refusal.rs` (`IngestError::Store(bedrock::BedrockError)`), and in `schema.rs`, whose two `jsonschema` calls validate readings on behalf of export and ingest.
- Cargo ignores `default-features = false` on a member's dependency line when the workspace declaration does not set it, with a warning.
  `domain` declares no default features, so only the `bedrock` declaration needs the flag.
- Cargo compiles what a manifest declares, not what a caller reaches.
  `domain` declares `rand` and uuid's `v4` for its constructors, and `now_ms` needs chrono's `now` — a feature `domain` never enabled itself and received only through other crates' feature unification (`specta`'s chrono support among them).
- `wasm32-wasip1` has a complete standard library: randomness, the clock, and standard error are WASI imports a host serves.
  `wasm32-unknown-unknown` has no host: `getrandom` ≥ 0.3 refuses to compile for it at the crate level, and its documented workaround (`wasm_js`) binds the module to a JavaScript host through `wasm-bindgen`, whose describe functions are exports and therefore survive linking whether or not anything calls them.
- The entropy and clock paths into a wasm32 build of the grammar, from `cargo tree`, were: `rand` (direct from `domain`), `uuid` with the workspace's `v4` and `js` features (`domain`, `bedrock-core`, `specta`, `transload` — `js` is the only path to `wasm-bindgen`), and `ahash` under `jsonschema`.
  Chrono is clean: `specta` enables its `clock` feature with defaults off, so no `wasmbind`, and `iana-time-zone` is a unix-only dependency.
- Every use of entropy or the clock in `domain` is a constructor, a `Default`, a builder default, the `touch` hook, or one `From<String>` conversion that mints an option id.
  Nothing in parsing, serialization, comparison, or derivation from a stable key touches either.
- `typed_builder` lets a field opt out of a struct-level `field_defaults(default)` with `builder(!default)`, which is what makes a gated `Default` survivable: the field becomes required instead of defaulting.
- No wasm target was installed in the pinned toolchain before this change; adding one rebuilds the devenv toolchain.
- House rules: every task lands hook-green; the per-commit hook runs `cargo nextest run --workspace` on the host with no feature flags, so a target-gated regression can only be caught by an explicit `--target` check.

## Goals / Non-Goals

**Goals:**

- `cargo check -p domain --features syntax --target <T>` and `cargo check -p transload --no-default-features --target <T>` succeed for both `wasm32-unknown-unknown` and `wasm32-wasip1`, with no `turso_core`, `getrandom`, `rand`, or `wasm-bindgen` in either graph.
- A probe module linking the grammar and the data model, built for `wasm32-unknown-unknown`, has an empty import section.
- `cargo build --workspace` and `cargo nextest run --workspace` are unchanged in outcome on the host, and the `store` round-trip over `getting-started` passes.
- Every crate that reaches the engine or mints says so in its own manifest.

**Non-Goals (design-level, beyond the proposal's):**

- No public-API stability promise for the pure build.
  Which items `transload` exposes without `store`, and which `domain` items exist on wasm32, is whatever the gates leave today; a later change may rearrange it.
- No attempt to make `store`, `cartograph`, or `outpost-*` buildable without the engine or for wasm32.
  They open databases and mint; that is their job.
- No replacement for what wasm32 loses.
  A wasm32 consumer that needs a fresh identifier has a design bug, and the compiler is the right place to hear about it.

## Decisions

### D1 — The pure mode is the façade's, not `domain`'s (2026-10-30)

`bedrock` gains a default-on `engine` feature.
Off, the crate is the `bedrock-core` re-exports plus the derives — exactly the surface the derives' `::bedrock::…` paths and `domain`'s hand impls need.
`domain` depends on the façade with no features.
Every crate that opens a database enables `bedrock/engine` through its own edge.
A grammar-only graph — `transload` (no `store`) → `domain` → `bedrock` (no `engine`) → `bedrock-core` — never names the engine.

The engine code lives under `src/engine/`, declared from `lib.rs` as one module behind the feature, with `lib.rs` re-exporting its public items so every `bedrock::…` path stays where it was.
The condition therefore appears twice — on the module and on the re-export block — and the directory, not an attribute on each item, is what says "this is engine code".
`lib.rs` keeps mapping out the whole module tree, which is the house shape.

- **Rejected — a `cfg` on every engine module and re-export in `lib.rs` (this change's first cut):** the same seam expressed eleven times, and nothing in the source tree showed where the engine began.
- **Rejected — `cfg_attr`-gating the derives in `domain`:** eight files of conditional attributes (and the `#[bedrock(id)]` helpers, which only exist when the derive is applied) to express "no engine", when the engine was never `domain`'s dependency to begin with.
  The façade's own docstring says the engine lives behind its boundary; this makes that true at the feature level.
- **Rejected — a `domain-core` split:** the persistence impls are derives on the type definitions; the orphan rule pins them to the crate that owns the types.
- **Rejected — compiling the engine for WebAssembly:** even where it links, an embedded database inside a module that must not open one is the wrong shape, and the size alone defeats the purpose.

### D2 — Features are named at the member, not inherited (2026-10-30)

The workspace declaration for `bedrock` sets `default-features = false`; `store`, `outpost-hold`, and `transload` enable `engine` explicitly, and `domain` enables nothing.
The workspace declaration for `uuid` drops to `serde`; `domain`, `cartograph`, and `xtask` — the crates that call `new_v4` — enable `v4` on their own lines, and `js`, which bound uuid to a JavaScript host for randomness and timestamps and which nothing in the repository targets, goes.
This is the only way cargo allows a member to opt out of a workspace dependency's defaults, and it has a benefit of its own: the manifests now state which crates reach the engine and which mint, instead of every crate reaching both by inheritance.

- **Rejected — leaving the workspace lines alone and opting out per member:** cargo ignores it and warns; the engine and the entropy stay in the graph.
- **Rejected — a second façade crate without the engine:** two names for one vocabulary, and the derives' emitted paths can only name one of them.

### D3 — `transload`'s seam is `store`, and it covers validation (2026-10-30)

`store` (default on) gates `export`, `ingest`, `IngestError::Store`, the reading-validation functions in `schema` (the `jsonschema` call sites), and the `bedrock` and `jsonschema` dependencies, which become optional.
The `_schema.json` face — building the Draft-07 document from a Chart and reading the canonical Chart back out of `x-waypoints` — stays in the pure build; only validating instances against it needs the validator.
The refusal _types_ stay unconditional, since they are part of the grammar's vocabulary; only the variant that wraps an engine error is gated.
As in `bedrock`, the gated code has its own directory: `export` and `ingest` live under `src/store/`, declared from `lib.rs` as one module behind the feature and re-exported from there, so the seam is visible in the tree and the public paths do not move.

- **Rejected — gating `jsonschema` separately from `store`:** nothing in the pure build needs it, and nothing in the default build can do without it; one seam is enough.
- **Rejected — gating `refusal` wholesale:** the malformed-Bundle kinds describe grammar failures and are useful to a grammar-only consumer.

### D4 — Minting and the clock are gated by target, not by feature (2026-10-31)

A plugin never mints an identifier and never reads the clock: it reports the identifiers its source already has, and the core mints at ingest.
So on a `wasm32` target those capabilities should not exist, and the cheapest true statement of that is a target condition.
`#[cfg(not(target_arch = "wasm32"))]` goes on every random constructor (`MarkID::new`, `TerrainID::new`, the four sync identities, `Base36ID::new` and the `ChartID`, `FeatureID`, and `GlyphID` wrappers), every `Default` that mints, `now_ms` and its re-exports, the `touch` hook and the `bedrock(before_save = touch)` attribute that names it, the `rand` imports, and the `From<String> for SelectOption` that mints an option id.
The manifest moves `rand`, `uuid/v4`, and `chrono/now` into a `cfg(not(target_arch = "wasm32"))` target table, so a wasm32 build of `domain` declares no entropy and no clock at all.
`Base36ID::derived`, `parse`, and every serde, `Display`, `FromStr`, and comparison impl stay: none needs entropy.

`target_arch = "wasm32"` rather than `unknown-unknown` alone, because a plugin never mints on any WebAssembly target and the gate should say what it means.

- **Rejected — a `mint` feature (this change's earlier design):** a feature is re-enabled by unification whenever any crate in the build turns it on, which under `cargo nextest run --workspace` is every crate; a target condition cannot be, needs no name, and needs no member to opt in or out.
- **Rejected — moving minting out of `domain` into the crates that are the core:** the same cut made as a crate boundary; a far larger refactor for the same result, and the constructors are where the types are.
- **Rejected — leaving minting in and neutering it through a deterministic host on `wasip1` (the plugin change's earlier D6):** it works, but it keeps dead minting code in every plugin, keeps an entropy source in the graph, and makes the sandbox a policy of the host rather than a property of the build.

### D5 — Builders require on wasm32 what they mint on the host (2026-10-31)

Where a builder fills an id or timestamp field by minting or by reading the clock — `Mark`'s `id`, `created_at`, and `updated_at`; `Terrain`'s `id`, `src_id`, and `dst_id`; `Landmark`'s `chart`; `Chart`'s `id`; `Feature`'s `id`; `SelectOption`'s and `SelectGroup`'s `id` — the attribute that does so is applied only off wasm32, and on wasm32 the field carries `builder(!default)` wherever a struct-level `field_defaults(default)` would otherwise reach for the gated `Default`.
A wasm32 build therefore cannot construct one of these without supplying the field, which is the right failure: a plugin constructs Fragment types, never a store entity, and if it ever tries, the compiler says so at the call site.
Host builders are byte-for-byte what they were.

- **Rejected — a non-minting `Default` on wasm32 (a nil UUID, an empty base-36 id):** a value that looks like an identity and is not one is the exact bug the gate exists to make impossible.

### D6 — Two targets, both required (2026-10-30, revised 2026-10-31)

This change is proven by compiling for WebAssembly because that is the consumer the pure build exists for and because no host target can show that the engine, the entropy, or the host bindings are truly gone — on a host they compile whether or not anything needs them.
Both targets are checked.
`wasm32-unknown-unknown` is the proof that the grammar is a sealed sandbox: nothing in the graph can ask a host for anything, which a probe module's empty import section demonstrates.
`wasm32-wasip1` is checked too, because the standard library there serves its own startup through WASI and a plugin runtime may prefer it for its standard error; what such a module imports is recorded so the plugin change can choose with the list in hand.

This decision began as an experiment — `unknown-unknown` investigated, `wasip1` required — on the prediction that `unknown-unknown` would fail on the data model's randomness and clock.
The experiment's findings, in the Outcome, confirmed the prediction precisely and measured exactly what to gate; D4 and D5 are that measurement applied.

- **Rejected — `wasip1` only:** proves the engine is gone and nothing about entropy, since the standard library there has it.
- **Rejected — proving the split on the host only:** `cargo tree -i turso_core` on a host graph shows the dependency gone, but not that what remains compiles for a target without an engine.

## Outcome

Recorded on the pinned toolchain (rustc 1.98.1) with both WebAssembly targets installed.
Two rounds: the experiment that measured the problem, then the proof after D4 and D5.

### Before gating (2026-10-31, the experiment)

**`wasm32-wasip1`, engine gated only.**
`cargo check -p domain --no-default-features --features syntax --target wasm32-wasip1` compiled (14 s cold); so did the same for `transload`; neither graph contained `turso_core`.
No transitive dependency needed gating beyond D3's `jsonschema`.

**`wasm32-unknown-unknown`, as is: fails, as predicted.**
Both checks stopped in `getrandom` 0.4.3 with its own `compile_error!`: "The wasm32/64-unknown-unknown are not supported by default; you may need to enable the `wasm_js` crate feature."
Reached through `rand` 0.10 (direct from `domain`) and through `uuid` 1.26 with the workspace's `v4` (`domain`, `bedrock-core`, `specta`, `transload`).

**The documented recipe: the cfg alone is not enough.**
`RUSTFLAGS='--cfg getrandom_backend="wasm_js"'` failed identically.
`getrandom` 0.4 needs both the cfg and its `wasm_js` feature, and no workspace crate enabled the feature (`uuid`'s `js` does not reach it); a direct `getrandom = { version = "0.4", features = ["wasm_js"] }` dependency in the probe supplied it.

**The probe, before gating.**
The probe (a `cdylib` outside the workspace, depending on `domain` with `syntax` and `transload` with no features, exporting one function that deserializes and re-serializes a `Content` and calls `layout::chart_dir`; release profile, `opt-level = "s"`, `lto`, `panic = "abort"`) needed the workspace lockfile and the workspace's `[patch.crates-io]` entry for `specta-macros` — without the patched derive, every `specta::Type` impl in `domain` silently vanishes.
A crate inside the workspace inherits both.

`wasm32-unknown-unknown` with the recipe: compiled, 285 558 bytes, three imports —

```
(import "__wbindgen_externref_xform__" "__wbindgen_externref_table_grow" (func (param i32) (result i32)))
(import "__wbindgen_externref_xform__" "__wbindgen_externref_table_set_null" (func (param i32)))
(import "__wbindgen_placeholder__" "__wbindgen_describe" (func (param i32)))
```

— `wasm-bindgen`'s raw-module placeholders, which only its CLI post-processing resolves, so a plain runtime cannot instantiate the module at all.
Its describe exports named what the glue would bind: `__wbg_getRandomValues_…` (uuid's `v4`, through `getrandom`) and `__wbg_now_…` (uuid's timestamp support under `js`), plus `BigInt`/`Number`/`Symbol` boilerplate.
Both survive linking because describe functions are exports.
The prediction held and was sharper than stated: the target compiles with the recipe, but the recipe's output is a JavaScript-hosted module, and the two things it asks the host for are precisely minting and the clock.

`wasm32-wasip1`, no configuration: compiled, 296 873 bytes, five imports (`environ_get`, `environ_sizes_get`, `fd_write`, `proc_exit`, `random_get`), no `_initialize` export.

### After gating (2026-10-31, the proof)

With D4 and D5 applied and the workspace `uuid` trimmed:

- `cargo check -p domain --features syntax --target wasm32-unknown-unknown`: compiles.
- `cargo check -p transload --no-default-features --target wasm32-unknown-unknown`: compiles.
- The same two for `wasm32-wasip1`: compile.
- `cargo tree -p transload --no-default-features --target <T> -i <crate>` for `getrandom`, `wasm-bindgen`, `rand`, and `turso_core`, on both targets: nothing to print, every time.
- On the host, `cargo tree -p domain -e normal -i getrandom@0.4.3` still names `rand` and `uuid`: minting is untouched where it belongs.

The probe, rebuilt with no configuration and no `getrandom` dependency of its own:

`wasm32-unknown-unknown`: compiles; 268 092 bytes; import section **empty**; exports `memory` and `probe`.
A sealed sandbox, as the data model stands.

`wasm32-wasip1`: compiles; 296 232 bytes; imports, verbatim:

```
(import "wasi_snapshot_preview1" "environ_get" (func (param i32 i32) (result i32)))
(import "wasi_snapshot_preview1" "environ_sizes_get" (func (param i32 i32) (result i32)))
(import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))
(import "wasi_snapshot_preview1" "proc_exit" (func (param i32)))
(import "wasi_snapshot_preview1" "random_get" (func (param i32 i32) (result i32)))
```

Unchanged from before gating, which settles what `random_get` is: not minting — nothing in this graph can mint — but the standard library seeding a `HashMap`, which domain's grouped-select readings carry.
A `wasip1` host that wants determinism must serve that one call with a fixed stream; an `unknown-unknown` host serves nothing, because the standard library there uses fixed hash keys.

**What the Outcome decides.**
The plugin change (`transload-sdk`) targets `wasm32-unknown-unknown`: the sandbox is a property of the build, the import-section check is "empty", and the deterministic-host apparatus (fixed clock, seeded random bytes, call counters) is unnecessary.
What it loses is standard error — a panic is a bare trap — which the SDK can recover with a panic hook writing into a guest buffer the host reads back.
`wasip1` remains available with the five-import allowlist above should a runtime want it.

**Measured.**
Cold `cargo check` of `domain` for a WebAssembly target: about 14 s; warm, under 4 s.
The probe's release build: about 13 s warm for `unknown-unknown`, about 8 s for `wasip1`.

**Fixed on the way.**
Before this change, `cargo nextest run --workspace` failed four tests: `transload`'s `bundle::tests::builtin_charts_keep_their_dotted_name`, its two ingest tests that resolve a builtin Chart, and `store`'s `the_collection_round_trips_through_the_store`, all because `layout::chart_dir` formatted the Chart through `Display`, which strips `builtin.` (`library/page/` where `library/builtin.page/` is expected).
It now reads the inner value through `AsRef`.

**Pre-existing, not this change.**
`cargo deny check advisories` fails on twelve advisories (`crypto-bigint`, `ctutils`, `pulldown-cmark`, `rsa`, `rustls-webpki`, `tokio`, `webpki-roots`, `zstd`, and dependents), none target-specific and all published after the file's last triage; licenses, bans, and sources pass, and the new targets surface nothing of their own.

## Risks / Trade-offs

- [A `domain` consumer on the host relied on a builder default this change gates on wasm32] → none can: every gate is target-conditional, and the host build is unchanged, which the whole-workspace build and test prove.
- [A new minting or clock site is added to `domain` without its gate] → the WebAssembly checks fail to compile or the probe's import section stops being empty; both are in the gate, and the plugin change adds the import check as a per-commit test.
- [A transitive dependency of the grammar starts needing a host on wasm32] → the inverse trees and the probe's import section are the detectors; the fallback is gating it where it belongs.
- [Feature unification hides a gating mistake under `nextest --workspace`] → every WebAssembly verification runs with `-p` and `--target`, where unification cannot reach, and the gates are target conditions, which unification cannot flip.
- [`cargo deny` surfaces advisories or duplicates on the new targets] → it surfaced none of its own; the pre-existing advisories are recorded above and left for their own triage.
- [The `crash-victim` binary or the Android allocator is forgotten] → both are gated and named in the task; `cargo check -p bedrock --no-default-features` fails if the binary is not.

## Migration Plan

1. The toolchain targets land first, alone, so the checks they enable can be run against every later commit.
2. `bedrock/engine` with the workspace declaration and the three member lines, as one commit: the workspace line and the member lines cannot land separately without a warning-only broken state in between.
3. `transload/store` with the `chart_dir` fix, each with its `wasip1` check.
4. The workspace `uuid` trim with the three `v4` lines, `domain`'s target table, and the gates, as one commit: the manifest and the source must move together or the host build loses minting.
5. Rollback: every feature defaults to today's behavior, and each commit reverts cleanly on its own.

## Open Questions

- Whether `cargo deny` needs the WebAssembly targets in its graph at all, or whether the sync task should learn to exclude verification-only targets.
  Deferrable; the targets surfaced nothing, so the question is moot until one does.
