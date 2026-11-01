## ADDED Requirements

### Requirement: The storage vocabulary is usable without the engine

The storage vocabulary — collection specifications, field encoding, identifier and document traits, and the derives that implement them — SHALL be usable by a crate that links no database engine.
A crate that only defines collections, or only depends on one that does, SHALL be able to exclude the engine from its dependency graph while keeping the vocabulary and the derives.
The engine SHALL remain the default for every crate that does not opt out.

#### Scenario: The data model links the vocabulary alone

- **WHEN** the data-model crate is checked with the engine excluded
- **THEN** it compiles
- **AND** its dependency graph contains no database engine

#### Scenario: Crates that open databases are unaffected

- **WHEN** the workspace is built and tested as before
- **THEN** every crate that opens a database still does, with no behavior change

## Decisions

- **2026-10-30 — The engine is a feature of the façade, on by default.**
  The façade already separates "the vocabulary the derives resolve against" from "the turso-backed engine" in its own documentation; this makes the separation a build-time fact.
  Rejected: conditional derive attributes in the data model (noise in eight files for a dependency the data model never had); a second façade crate without the engine (two names for one vocabulary, and the derives can emit only one).
  See design D1, D2.
