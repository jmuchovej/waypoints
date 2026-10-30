## ADDED Requirements

### Requirement: The Bundle grammar is usable without the persistence engine

The Bundle grammar — layout and manifest, frontmatter, identity comments, sidecar lines, the Chart schema document, link names, and the refusal vocabulary — SHALL be usable by a consumer that links no database engine, including a consumer compiled for a WebAssembly target.
Export, ingest, and the validation of readings against a Chart schema, which need a store or a schema validator, SHALL be separable from the grammar they use, and SHALL remain present by default.

#### Scenario: The grammar compiles for WebAssembly without an engine

- **WHEN** the Bundle crate is checked for `wasm32-wasip1` with export and ingest excluded
- **THEN** it compiles
- **AND** its dependency graph contains no database engine and no schema validator

#### Scenario: Export and ingest are unchanged by default

- **WHEN** the application is built as before
- **THEN** export and ingest are present
- **AND** the committed getting-started Bundle round-trips exactly as before

## Decisions

- **2026-10-30 — One seam, `store`, covering export, ingest, the store error, and reading validation.**
  Nothing in a grammar-only build needs the validator and nothing in the default build can do without it, so they gate together; the schema document itself stays in the grammar because building it needs no validator.
  Rejected: a separate validator feature (a second seam with no consumer on either side); gating the refusal vocabulary wholesale (malformed-Bundle kinds describe grammar failures a grammar consumer wants).
  See design D3.
