## Why

Waypoints can leave (`transload::export`) and come back (`transload::ingest`), but nothing can arrive from anywhere else: the only input the ingester accepts is a Bundle carrying declared `id`s, and the one path that once parsed foreign markdown — the legacy `importer` crate — is gone from the workspace.
The importer plugin system was designed in August as two documents, the projection spec's §9 (identity resolution, the ledger, re-import semantics) and a WASM plugin contract (`importer-abi`, Draft 0.1, copied into this change directory as `importer-abi.md`).
The first drained into the archived `transload-pipeline` change as binding context; the second was never pulled in, and the Bundle grammar it was written against has since moved under it — identity comments are a bare id, mentions are title wiki-links, `marks.jsonl` is mandatory, the edge tiers collapsed to two.

This change lays the plugin-facing half of that system: the typed Fragment a plugin emits, the ABI a plugin module exposes, the SDK that generates it, the conformance harness that proves a plugin deterministic, and a fixture plugin that exercises all of it.
It builds on the archived `grammar-only-builds` change, which made `domain` and `transload` compile for `wasm32-unknown-unknown` with no engine, no entropy, no clock, and no host binding in their graph — a probe module built from them imports nothing at all.
That is the sandbox this change inherits: a plugin is sealed by its build, and this change adds nothing to that build but the plugin surface.
The host-facing half — the ledger, Fragment ingestion, the step where a user maps what a plugin discovered onto Charts, the import report, the WebAssembly runtime embedded in the app — is out of scope: a Fragment grammar is what that work consumes, so it has to exist first, and a plugin has to be writable and testable before any runtime exists to load it.

## What Changes

- **The Fragment grammar**, in `transload`: the typed form of what an importer emits.
  A Landmark Fragment, Content Fragment, Terrain Fragment, and the readings and feature kinds beneath them mirror their data-model counterparts one for one, with a **source key** in every slot the data model fills with a minted identifier (`MarkID`, `ChartID`, `FeatureID`, `GlyphID`), and reuse `domain` types unchanged everywhere else — `Content`, `RichText`, `Icon`, the scalar readings.
  A Fragment carries `src` and never `id`; the existing rule that a Bundle carries `id` and never `src` is untouched.
- **Protocharts, not Charts.**
  A plugin never emits or binds a Chart.
  It emits a Protochart — the structure it discovered in the source (a Notion database's columns, a Logseq page's properties): feature keys, kinds, select options, and the labels a person needs to recognize it — and binds each Landmark to a protochart key, with readings keyed by the protochart's feature keys.
  A Landmark may also bind to no protochart, carrying no readings.
  Resolving protocharts to Charts is not the plugin's concern and not this change's.
- **Fragment versioning** mirrors the Bundle manifest's two-axis scheme: a `fragment_format` semver for the envelope, and the `domain` version a plugin was compiled against, both stamped by the SDK, plus a census of the variants a run actually emitted, computed at finalize.
- **The importer ABI** (`interchange/abi`): a plugin is a `wasm32-unknown-unknown` module exporting exactly eight functions (`wp_abi`, `wp_alloc`, `wp_free`, `wp_manifest`, `wp_init`, `wp_entry`, `wp_finalize`, `wp_panic`) with their signatures frozen, exporting its memory, and **importing nothing** — the import section is empty.
  All data crosses as guest-owned buffers; `u64` returns pack `(ptr << 32) | len`; payloads are JSON; every fallible call returns a `Result` envelope and a trap aborts the run.
  Entries are fed in a defined order: the manifest's marker files first, then every other entry in sorted path order.
  Diagnostics ride in-band in every output.
  The ABI version is a frozen major; payload evolution within it is additive, and a reader ignores fields it does not know.
- **Determinism is a property of the build.**
  A plugin's output for a given configuration and entry sequence is byte-identical across runs, and nothing in the module can reach a host: the data model has no minting and no clock on wasm32, the target's standard library has no entropy and fixed hash keys, and the module declares no imports.
  There is no clock to fix, no randomness to seed, and nothing to count; a host that instantiates the module with an empty import table has given it everything it can ask for.
- **A trap carries its message.**
  The target has no standard error, so the SDK installs a panic hook that stores the panic's message and location in guest memory, and `wp_panic` hands it back; a host reads it after a trap and folds it into the trap report.
- **The SDK**, behind a `sdk` feature of `transload`: an `Importer` trait (`manifest`, `init`, `entry`, `finalize`) and a macro that generates the eight exports, owns allocation and the packed return, installs the panic hook, and serializes through serde.
  The plugin's manifest is authored as a TOML file the macro embeds and serves as JSON, with the version stamped from the crate and the required fields (identity, display name, icon, namespace, source shape) checked.
  The options schema the host renders a configuration form from is generated from the plugin's configuration type as JSON Schema Draft-07, not hand-written.
  At finalize the SDK checks that every Landmark's protochart key, reading key, and select value is defined by the run, and computes the census.
  A plugin is plain Rust, so its own tests run natively.
- **A conformance harness**, behind a `harness` feature of `transload`: fixtures in, expected outputs out, byte-compared, run natively through the trait and again as a module under wasmi with an empty import table, with both runs required to produce the same bytes.
  The module run also asserts the export set and signatures, and instantiation itself is the import check: a module that declares any import cannot be instantiated.
- **A fixture plugin**, a workspace member under `crates/transload/fixtures/plain-markdown`, that turns a directory of plain markdown into one Landmark Fragment per file with a few blocks and no protochart.
  It exercises the macro, the manifest, the panic hook, the harness, and the wasm build; its tests own every check that must run against a plugin's own dependency graph.
  It is a test fixture, not a product importer.
- **Spec organization:** this change mints an `interchange` domain tree.
  The ABI lands at `interchange/abi`.
  `transload` relocates to `interchange/transload` when this change is archived — a directory move plus the registry entry in `openspec/config.yaml`, done by hand since archiving does not move capabilities — and its delta here is written against the current path.
  Per-source importer specs take the shape `interchange/importer/<source>`; none lands here.

## Capabilities

### New Capabilities

- `interchange/abi`: the contract between a host and an importer plugin module — module shape and the empty import section, export signatures and buffer convention, result envelope and the panic message, lifecycle and entry order, manifest, configuration schema and validation, determinism by construction, in-band diagnostics, finalize-time consistency, versioning — and what the SDK and conformance harness guarantee about a plugin built with them.

### Modified Capabilities

- `transload`: gains the Fragment grammar — source-keyed identity with `id` reserved, the mirrored entity shapes, protocharts and the optional Landmark binding, key scoping, references by key including forward references, nothing the core owns, no reading-kind terrain lines, canonical serialization, and the envelope's versioning.
  The Bundle-side requirements are unchanged; "Export-Bundles carry declared identity and never source keys" stays as written and gains its mirror.

## Non-goals

- **No ledger, no Fragment ingestion, no protochart-to-Chart mapping, no import report, no re-import semantics.**
  The §9.1–9.3 rules and design D4 of `transload-pipeline` remain binding for that work; nothing here resolves a source key or a protochart.
  The ingester keeps refusing `src` in Bundle position.
  Three questions that work must answer are recorded as open in `design.md`: where unmapped readings go when a protochart falls back to a Chart without those features, how the ledger keys protochart, feature, and option mappings, and whether reading-kind terrain edges are ever materialized.
- **No WebAssembly runtime in the application**, no fuel or memory limits, no Tauri command, no configuration-form UI, no plugin discovery or installation.
  The harness is the only host this change builds.
- **No host imports of any kind**, WASI included.
  A plugin that needs one is not a plugin; the `wasip1` import list recorded in `grammar-only-builds` is what a future runtime would have to serve if it ever chose that target, and nothing here chooses it.
- **No Logseq importer**, nor any product importer.
  The fixture plugin is the only plugin.
- **No assets.**
  The draft's asset marker is omitted: no asset model exists in `domain`, and the Bundle reserves `assets/` without implementing it.
- **No non-Rust SDK and no typing artifact for one.**
  The only typed surface is the Rust Fragment.
- **No `TerrainID`, order strings, content hashes, or clocks in a Fragment.**
  All four are the core's.
- **No change to the dialect, the Bundle layout, any Bundle-side requirement, or how builtin Charts are identified.**
  The conformance suite and the committed `getting-started` Bundle are the gate for the first three; the `builtin.` prefix never appears in the Fragment grammar, so the fourth is untouched by construction.
- **No edits to the Notion pages.**
  `importer-abi.md` in this directory is the historical record, pulled in the way `projection-spec.md` was.

## Impact

**Code**

- `crates/transload` — `sdk` feature (default off) around the trait, the macro, the manifest loader, the options-schema generator, and the panic hook; `harness` feature (default off) around the native and module harness backends; a new `fragment` module and an `abi` module (manifest, diagnostics, envelopes).
  `exclude = ["fixtures"]` on the package so the nested fixture crate is not packaged with it.
  Public surface grows; nothing existing changes.
- `crates/transload/fixtures/plain-markdown` — new workspace member, `cdylib` + `rlib`, depending on `transload` with `sdk` only and, for its tests, `harness`.
- `openspec/config.yaml` — the `interchange` tree.

**Dependencies**

- `toml` (already a workspace dependency) and `schemars` (already in the lockfile through other crates) become `transload` dependencies under `sdk`; both are pure Rust and reach a plugin's wasm build.
  `schemars` brings a derive macro, so a plugin build gains one proc-macro crate.
- `wasmi` (new; MIT/Apache-2.0; default features off plus `std`) under `harness`, never in a plugin build.
  `wasmparser` is not needed: wasmi reads a module's export and import sections itself.
  Nothing else: with no host functions to provide, the module backend needs no random source and no clock.

**Systems**

- The fixture crate's tests run a nested `cargo build --target wasm32-unknown-unknown` of the fixture into its own target directory, then load the artifact under wasmi.
  Under `cargo nextest run --workspace` that is one more wasm build per hook run, of the fixture and every dependency of `domain` and the grammar for a second target; `grammar-only-builds` measured a warm release build of a comparable probe at about thirteen seconds, and the task records the figure for the fixture.
- `cargo deny` sees `wasmi` and the direct `schemars` and `toml` edges; nothing in them is new to the lockfile's license set.

**Specs**

- Creates `interchange/abi`; amends `transload`; registers the `interchange` tree and records the manual relocation of `transload` for archive time.
