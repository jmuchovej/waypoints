## ADDED Requirements

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

A Mark referenced from *within* another Mark's block content — a mention, an
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

## Decisions

### 2026-08-06 — Landmark content is Terrain-only; the child list is removed

A Landmark's content is expressed solely by Terrain child edges and their
fractional order index. The child-reference field that Landmark data previously
carried is deleted rather than retained and documented as inert.

_Rationale:_ the field had no writer and no reader. Projection already derived
content exclusively from child edges, so the field was a second nominal
authority that could only ever disagree with the real one. Under sync that
asymmetry is a silent divergence — one device populating what another ignores,
with no conflict to detect. Deleting it makes the invariant structural rather
than documentary.

_Consequence accepted:_ Landmark records persisted before this change still
carry the key in their stored form. It is inert, ignored on read, and absent
from records written afterward. No backfill is performed; the two forms coexist
without observable difference.

_Rejected:_ retaining the field and annotating it as vestigial. Accurate, but a
serialized field is a standing invitation to treat it as meaningful; the next
contributor re-opens the question. A field nothing operates on should not exist
to suggest otherwise.

_Rejected:_ removing the reference type the field held. That type answers a
question the codebase still has — how a Mark referenced *inside* block content
should be displayed — which containment edges do not address. It is early, not
dead, and the second requirement above is why the two concerns stay separate.

_Origin:_ established by the op-log-completeness change, whose investigation
found the asymmetry while auditing which mutations are operation-shaped. The
`editor` capability references this invariant for projection; this capability
owns it.
