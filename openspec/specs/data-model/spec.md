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
