## ADDED Requirements

### Requirement: The Bundle grammar is usable without the persistence engine or a source of entropy

The Bundle grammar — layout and manifest, frontmatter, identity comments, sidecar lines, the Chart schema document, link names, and the refusal vocabulary — SHALL be usable by a consumer that links no database engine and no source of entropy, including a consumer compiled for a WebAssembly target.
Export, ingest, and the validation of readings against a Chart schema, which need a store or a schema validator, SHALL be separable from the grammar they use, and SHALL remain present by default.
A Bundle directory name SHALL spell a Chart id exactly as the Bundle carries it, prefix included.

#### Scenario: The grammar compiles for WebAssembly without an engine or entropy

- **WHEN** the Bundle crate is checked for `wasm32-unknown-unknown` with export and ingest excluded
- **THEN** it compiles
- **AND** its dependency graph contains no database engine, no schema validator, no system-randomness crate, and no JavaScript-host binding

#### Scenario: A module built from the grammar imports nothing

- **WHEN** a module that links the grammar and the data model is built for `wasm32-unknown-unknown`
- **THEN** its import section is empty

#### Scenario: Export and ingest are unchanged by default

- **WHEN** the application is built as before
- **THEN** export and ingest are present
- **AND** the committed getting-started Bundle round-trips exactly as before

#### Scenario: A builtin Chart keeps its prefix in its directory name

- **WHEN** a Landmark on the builtin page Chart is exported
- **THEN** its file lives under `library/builtin.page/`

## Decisions

- **2026-10-30 — One seam, `store`, covering export, ingest, the store error, and reading validation.**
  Nothing in a grammar-only build needs the validator and nothing in the default build can do without it, so they gate together; the schema document itself stays in the grammar because building it needs no validator.
  Rejected: a separate validator feature (a second seam with no consumer on either side); gating the refusal vocabulary wholesale (malformed-Bundle kinds describe grammar failures a grammar consumer wants).
  See design D3.
- **2026-10-31 — Directory names read the Chart id through `AsRef`, never `Display`.**
  `Display` is the Chart's name and strips `builtin.`, collapsing a builtin onto a custom Chart of the same name.
  Rejected: giving `Display` the full id (the name is what every other rendering wants).
