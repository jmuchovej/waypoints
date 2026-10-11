## ADDED Requirements

### Requirement: View projections address namespaced document keys

A view column or scope MAY declare a namespace alongside its path. The
namespace SHALL be emitted as the first JSON-path element, always quoted and
never split, so a top-level key containing dots (e.g.
`so.waypoints.importer.logseq`) addresses one key rather than a chain of
nested segments. A single-segment dotted path without a namespace SHALL
retain segment-splitting semantics (`readings.abc` → `$.readings.abc`),
byte-for-byte compatible with existing view declarations. Declarations
without a namespace remain valid; stored view specifications require no
rewrite.

#### Scenario: Namespaced key extracts

- **WHEN** a view column declares namespace `so.waypoints.importer.logseq`
  and path `id` over a document whose `extra` carries that namespace object
- **THEN** the view projects the namespace object's `id` value, verified
  against the real engine

#### Scenario: Legacy paths unchanged

- **WHEN** a view column declares path `readings.abc` and no namespace
- **THEN** the emitted JSON path is `$.readings.abc`, identical to the
  pre-change lowering

## Decisions

### 2026-08-18 — Namespace as a declared field, not path syntax

_Rationale:_ the lowering splits paths on dots by design, and namespaced
top-level keys are themselves dotted — the two conventions collide inside
one string. A separate optional field keeps both: explicit structure over
in-band escaping, and serde defaults keep every stored spec valid.
_Rejected:_ quote-aware or escape-aware path parsing inside the single
string (stringly-typed grammar where a field suffices); always quoting every
segment of every path (changes emitted DDL for all existing views to buy
nothing — least-change wins).
