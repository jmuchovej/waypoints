## ADDED Requirements

### Requirement: The data model is usable without a storage engine

The data model SHALL be usable — its values constructed, read, compared, and serialized — by a consumer that links no database engine, including a consumer compiled for a WebAssembly target.
The data model's dependency on the storage vocabulary SHALL NOT imply a dependency on the engine.
The data model SHALL declare every standard-library capability its own code uses in its own manifest, so that building it alone does not depend on which other crates are in the graph.

#### Scenario: The data model compiles for WebAssembly without an engine

- **WHEN** the data-model crate is checked for `wasm32-unknown-unknown` or `wasm32-wasip1` with its dialect feature
- **THEN** it compiles
- **AND** its dependency graph contains no database engine

#### Scenario: Behavior under default features is unchanged

- **WHEN** the application is built and tested as before
- **THEN** every data-model type, constructor, default, and derive behaves identically

### Requirement: Identifier minting and the clock do not exist on WebAssembly targets

On a `wasm32` target the data model SHALL provide no way to create a fresh identifier (`MarkID`, `TerrainID`, `ChartID`, `FeatureID`, `GlyphID`, or a sync identity) and no way to read the current time.
Every identifier type SHALL still parse, format, serialize, deserialize, and compare there; a deterministic derivation from a stable key SHALL remain available.
A builder that fills an identifier or timestamp field by minting or by reading the clock on a host target SHALL require that field on a `wasm32` target instead.
A `wasm32` build of the data model SHALL declare no source of entropy and no clock in its dependency graph.

#### Scenario: Minting does not compile

- **WHEN** code compiled for a `wasm32` target calls `MarkID::new()` or relies on an identifier type's `Default`
- **THEN** it fails to compile

#### Scenario: Identifiers still parse and derive

- **WHEN** a `wasm32` build deserializes a `MarkID` from its string form or derives a `FeatureID` from a stable key
- **THEN** it succeeds and the value equals the host build's

#### Scenario: No entropy in the graph

- **WHEN** the data-model crate's dependency graph for `wasm32-unknown-unknown` is listed
- **THEN** it contains neither a system-randomness crate nor a JavaScript-host binding

#### Scenario: Host builds mint as before

- **WHEN** the application is built for a host target
- **THEN** every constructor and default mints or stamps exactly as it did

## Decisions

- **2026-10-30 — The data model's manifest changes; 2026-10-31 — and its source gains target conditions, nothing else.**
  The data model depends on the storage façade for its vocabulary and derives only, so excluding the engine is a dependency-line change.
  Minting and the clock are gated by target rather than by a feature: a target condition cannot be re-enabled by feature unification, needs no name, and says what it means — a plugin never mints.
  Rejected: splitting a persistence-free core crate out of the data model (the persistence impls are derives on the type definitions and cannot live elsewhere); a `mint` feature (one more feature to keep straight across every member, and unification would turn it back on under a workspace build); moving minting into the crates that are the core (the same cut as a crate boundary, a far larger refactor for the same result).
  See design D1, D4, D6.
