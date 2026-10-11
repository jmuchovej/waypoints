## ADDED Requirements

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

## Decisions

### 2026-08-18 — `Chart.extra` returns; feature extra is Chart-owned

_Rationale:_ the transload pipeline requires `extra` parity across all three
residue-bearing entities — Chart residue must ride the Bundle's schema
documents, and there is nothing to ride until the field exists. Features
never had their own table, so a per-feature carrier would invent an entity;
keying feature extra by `FeatureID` inside the Chart's `extra` matches where
features already live (inside the Chart), and the two key populations —
4-character base-36 slots and dotted reverse-DNS namespaces — cannot
collide, enforced by reserving the slot pattern rather than trusting it.
_Rejected:_ a Feature-level `extra` field on each `Feature` (bloats every
schema document for a per-feature concern that is rare); a separate
feature-extra sidecar (violates residue-per-entity — the Chart is the
entity).

### 2026-08-18 — Timestamp stamping discriminates on write class

_Rationale:_ unconditional stamping conflates "this row was written" with
"this content was edited now." Bundle ingest is replication of domain facts,
and the `updated` instant is one of those facts; losing it to the wall
clock is a fidelity violation. The write class is already visible to the
persistence hook, so discrimination costs one match.
_Rejected:_ excluding `updated_at` from roundtrip equality (weakens fidelity
to dodge a one-line fix); a hook-bypassing write mode (real API surface for
a problem the write class already answers — revisit only if update-in-place
re-import needs it).

### 2026-08-18 — RichReference: soft-deleted from the surface, retained in-tree

_Rationale:_ the type answers a question the codebase still has — how an
inline reference displays (avatar, card, embed) — but nothing consumes it
yet, and an exported type invites premature use. Un-exporting keeps the
answer while removing the invitation; the display-mode concept is parked
for future embedding into Terrain edges and Charts.
_Rejected:_ deleting the module (discards a considered design the inline
reference case will need); keeping it exported (a public type with zero
consumers is documentation that lies about the API).
_Origin:_ layers on the op-log-completeness change, which removes the last
field that held it and retains the type; this delta tightens retained to
retained-but-private.
