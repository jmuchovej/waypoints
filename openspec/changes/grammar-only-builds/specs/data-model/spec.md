## ADDED Requirements

### Requirement: The data model is usable without a storage engine

The data model SHALL be usable — its values constructed, read, compared, and serialized — by a consumer that links no database engine, including a consumer compiled for a WebAssembly target.
The data model's dependency on the storage vocabulary SHALL NOT imply a dependency on the engine.
The data model SHALL declare every standard-library capability its own code uses (its clock among them) in its own manifest, so that building it alone does not depend on which other crates are in the graph.

#### Scenario: The data model compiles for WebAssembly without an engine

- **WHEN** the data-model crate is checked for `wasm32-wasip1` with its dialect feature and no storage engine
- **THEN** it compiles
- **AND** its dependency graph contains no database engine

#### Scenario: Behavior under default features is unchanged

- **WHEN** the application is built and tested as before
- **THEN** every data-model type, constructor, default, and derive behaves identically

## Decisions

- **2026-10-30 — The data model's manifest changes; its source does not.**
  The data model depends on the storage façade for its vocabulary and derives only, so excluding the engine is a dependency-line change.
  Rejected: splitting a persistence-free core crate out of the data model (the persistence impls are derives on the type definitions and cannot live elsewhere); gating identifier minting or the clock (sandbox hardening, and a source change this change does not make).
  See design D1, D4.
