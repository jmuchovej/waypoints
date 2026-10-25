# data-model Specification

## Purpose

TBD - created by archiving change op-log-completeness. Update Purpose after archive.

## Requirements

### Requirement: A Landmark's content is expressed solely by Terrain child edges

The ordered content of a Landmark — the blocks that constitute its body — SHALL
be expressed exclusively by Terrain child edges from that Landmark to those
Marks. Landmark data SHALL NOT carry a child list, a child-reference collection,
or any other field naming its contents.

Ordering among a Landmark's contents SHALL be carried by the fractional order
index on each child edge, not by position within any field of the Landmark.

Any derivation of a Landmark's content — projection, rendering, export, or
traversal — SHALL read Terrain child edges and SHALL NOT consult Landmark data
for containment.

#### Scenario: Containment is derived from edges

- **WHEN** a Landmark's content is derived for any purpose
- **THEN** the result is determined entirely by the Terrain child edges whose
  source is that Landmark
- **AND** no field of the Landmark's data contributes to the result

#### Scenario: Landmark data names no children

- **WHEN** a Landmark's data is inspected
- **THEN** it carries the Landmark's chart, title, icon, and readings
- **AND** it carries no field enumerating the Marks contained within it

#### Scenario: Ordering lives on the edge

- **WHEN** two Marks contained by the same Landmark are ordered relative to each
  other
- **THEN** the relative order is determined by the order index on their child
  edges
- **AND** reordering them changes only edge state, leaving Landmark data
  unchanged

#### Scenario: A stored child list is not an authority

- **WHEN** persisted Landmark data written before this invariant took effect
  still carries a child-reference key
- **THEN** that key is ignored on read
- **AND** it does not contribute to the Landmark's derived content

### Requirement: Referencing a Mark within content is not containment

A Mark referenced from _within_ another Mark's block content — a mention, an
inline card, an embedded reference — SHALL NOT thereby be contained by the
referencing Mark, and SHALL NOT produce a Terrain child edge.

Containment answers which Marks constitute a Landmark's body. An inline
reference answers which Mark some text points at, and how that pointer should be
displayed. These are distinct relationships and SHALL remain separately
expressed.

#### Scenario: An inline reference creates no containment

- **WHEN** block content references another Mark inline
- **THEN** no child edge is created between the two Marks
- **AND** the referenced Mark does not appear in the referencing Landmark's
  derived content

### Requirement: Extra is domain data on Charts, Marks, and Terrain edges

Charts, Marks, and Terrain edges SHALL each carry an optional, schema-less
`extra` value. `extra` is domain data: it persists, it exports, and it
round-trips unchanged. Namespaced keys (reverse-DNS, e.g.
`so.waypoints.importer.logseq`) partition `extra` between independent
writers.

Because Features have no independent existence outside their Chart,
feature-level extra SHALL live inside the owning Chart's `extra`, keyed by
`FeatureID`. Keys matching `^[0-9a-z]{4}$` within a Chart's `extra` are
reserved as feature slots; namespaced keys and feature slots coexist in one
object without ambiguity.

#### Scenario: Chart extra persists

- **WHEN** a Chart is saved with an `extra` value and read back
- **THEN** the value is identical

#### Scenario: Feature-keyed extra

- **WHEN** feature-scoped data is attached for feature `a1b2` of a Chart
- **THEN** it is stored under key `a1b2` in that Chart's `extra`
- **AND** a namespaced key in the same object is unaffected

### Requirement: Update timestamps stamp only on update-class writes

A Mark's `created_at` is set at construction and callers MAY set it
explicitly. `updated_at` SHALL be stamped by the persistence layer on
update-class writes only; insert-class writes (insert, bulk insert, upsert)
SHALL preserve the caller-provided value. Replication-style writers — ingest
restoring a Bundle's `updated` time — therefore write timestamps as domain
data; interactive edits keep stamping.

#### Scenario: Ingest-style insert preserves timestamps

- **WHEN** a Mark carrying explicit `created_at` and `updated_at` values is
  inserted
- **THEN** both values persist exactly as provided

#### Scenario: Interactive update stamps

- **WHEN** an existing Mark is written through an update
- **THEN** its `updated_at` reflects the write time

#### Scenario: Upsert does not refresh silently

- **WHEN** an existing Mark is written through an upsert carrying an explicit
  `updated_at`
- **THEN** the stored `updated_at` is the carried value, not the wall clock

### Requirement: RichReference is not part of the exported domain surface

The domain crate's public API and its generated TypeScript bindings SHALL NOT
export the inline-reference display type (`RichReference`). The type remains
in-tree, unexported, awaiting its use site.

#### Scenario: Bindings omit the type

- **WHEN** TypeScript bindings are regenerated from the domain crate
- **THEN** no `RichReference` type is emitted
