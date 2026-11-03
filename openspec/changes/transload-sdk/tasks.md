Every task lands with hooks green (clippy, cargo-check, cargo-test, treefmt).
This change stands on the archived `grammar-only-builds`: the `wasm32-unknown-unknown` target, `transload`'s `store` feature, and the data model's wasm32 gates are assumed present.
The acceptance gate is the fixture plugin: conformant under both harness backends, deterministic, a `wasm32-unknown-unknown` module with exactly the eight exports and an empty import section, and a trap that still carries its message.
Every check about the plugin's own dependency graph runs inside the fixture crate's tests through a nested cargo build, where workspace feature unification cannot reach.

## 1. Spec registry

- [x] 1.1 Add the `interchange` tree to `openspec/config.yaml` (`abi`, `transload`, and `importer/<source>` as the per-source shape) and note that `transload` relocates under it by hand at archive; verify `openspec validate transload-sdk --strict` passes.

## 2. Fragment grammar

- [x] 2.1 Add `transload::fragment` with `SourceKey`, `LandmarkFragment` (optional `protochart`), `ContentFragment`, `Protochart`, `FeatureFragment` (with `description` and `required`), `FeatureKindFragment`, `SelectOptionFragment`, `SelectGroupFragment`, `ReadingFragment`, `TerrainFragment`, `TerrainKindFragment` (`Child | Reference | Custom`), and `FRAGMENT_FORMAT`; verify serde round-trip tests per type pass, a Content Fragment carrying `id` is refused, and the same payload with an undefined field name deserializes.
- [x] 2.2 Add the mirror-exhaustiveness tests: a non-wildcard match from each of `Reading`, `FeatureKind`, and `TerrainKind` onto its Fragment counterpart, with `TerrainKind::Reading` as the named exclusion; verify they compile and pass, and that a mirror missing a variant fails to compile (a `trybuild` case over `Reading`, with its expected output committed).
- [x] 2.3 Add canonical serialization (unordered collections sorted at the boundary, millisecond timestamps, stable field order) and the census accumulator over Fragment values; verify an SDK-written output round-trips byte for byte and a census over a known Fragment set lists exactly the variants present.

## 3. ABI types, manifest, and schema

- [x] 3.1 Add `transload::abi` with `Diagnostic` (`level`, `msg`, `at`), the `Envelope` using `Diagnostic` as `err`, `Init`, `Output` (the entry shape), `FinalizeOutput` with census and summary, `Manifest` split into `Authored` and stamped fields, and `ABI = 1`; verify serde tests pass and unknown fields on outputs are ignored.
- [x] 3.2 Add the manifest loader under `sdk`: parse the authored TOML with `toml` (parse and serde features only, unknown fields refused), check required fields (`id`, `name`, `icon`, `namespace`, `source.kinds`), and merge the stamped fields (`version` from `CARGO_PKG_VERSION`, `abi`, `fragment_format`, `domain_version`); verify a valid document loads, a missing required field and an undefined field each fail naming the field.
- [x] 3.3 Add the options-schema generator under `sdk`: `schemars` with `SchemaSettings::draft07()` over the configuration type, asserting the emitted `$schema`; verify the generated schema for a two-field test configuration validates against the Draft-07 meta-schema and names exactly its fields.

## 4. Trait, macro, and panic hook

- [x] 4.1 Add the `Importer` trait under `sdk` (`type Config: DeserializeOwned + JsonSchema`; `manifest`, `init`, `entry`, `finalize` consuming `self`); verify a minimal in-test implementation compiles and its methods run natively.
- [x] 4.2 Add the panic capture under `sdk`: a hook installed once through a `Once` on the first export call that stores the panic's message and location in a guest-side static, and `wp_panic` returning it packed or `0`; verify on the host that a caught panic leaves its message and location readable through the hook's store, and under the module backend that `wp_panic` returns them after a trap.
- [x] 4.3 Add the `importer!` macro generating the eight `extern "C"` exports on wasm32, `wp_alloc`/`wp_free` over boxed slices, the packed `u64` return, the slot-index state handle inside the `ok` payload, JSON (de)serialization of every payload, the embedded TOML manifest, the finalize-time protochart checks, the census, and the hook installation, all as thin wrappers over `sdk::Runtime`; verify, in one integration-test binary holding one test importer, that the `Runtime` driven over bytes returns a manifest, an init result with a state, an entry output, and a finalize output, that the hook's store is empty before any panic, and that a reading on an undefined feature yields an `err` envelope at finalize while a mention to an undefined Mark key does not.

## 5. Harness

- [x] 5.1 Add `transload::harness` behind `harness` with the native backend: Draft-07 validation of the configuration against the generated schema (refusing before `init`), entry order with markers first then sorted paths, byte comparison against goldens with a readable diff on mismatch, and a two-run determinism check; verify it passes over the fixture plugin's fixtures and fails naming the entry when one golden byte is changed.
- [x] 5.2 Add the module backend: `wasmi` (default features off plus `std`, `validate`, and `auto-dispatch`) with an empty `Linker`, instantiation as the import check (an import fails instantiation and the error names it), the lifecycle through the real exports, export-set and signature assertions, parity assertions against the native backend, and after a trap a `wp_panic` read folded into the reported error; verify it against the fixture module in task 6.3.

## 6. Fixture plugin

- [x] 6.1 Add `crates/transload/fixtures/plain-markdown` as a workspace member (`cdylib` + `rlib`; `transload` with `sdk` only as a dependency and with `harness` as a dev-dependency; `exclude = ["fixtures"]` on `transload`'s package) implementing `Importer` per design D10 with its TOML manifest and a configuration type of `title_from` and `fail_on_entry`; verify its native unit tests pass and `cargo clippy --workspace --all-targets` is clean for the new crates.
- [x] 6.2 Commit its fixture directory and golden outputs (one expected JSON per entry plus the manifest and finalize outputs) under the crate; verify the goldens show Landmarks with no protochart, keyed blocks, a `[[name]]` mention keyed `page:<name>` that matches a fixture page and one that does not, and a `warn` diagnostic with entry and line.
- [x] 6.3 Add the fixture crate's graph tests: a nested `cargo build --release -p plain-markdown --target wasm32-unknown-unknown` into its own target directory, then the module backend over the artifact — exactly the eight exports plus `memory`, an empty import section, parity with the native run, and a `fail_on_entry` run that traps with `wp_panic` returning the message and location; verify all pass. Measured: the nested build takes about 23 s cold and 0.3 s warm; the module is 597 413 bytes under the workspace release profile (`opt-level = "s"`, LTO, stripped).

## 7. Close out

- [x] 7.1 Run the full gate: `cargo nextest run --workspace`, `cargo deny check`, and `cargo test -p transload --features sdk,harness`; verify all pass with the app build unchanged.
- [x] 7.2 Record the measured costs and module size in `design.md`; verify `openspec validate transload-sdk --strict` passes.
