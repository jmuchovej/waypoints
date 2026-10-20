## ADDED Requirements

### Requirement: Bundle layout carries only domain-level identifiers

An exported Bundle SHALL be a plain directory containing `manifest.json`,
`library/<ChartID>/<MarkID>.md` (one file per exported Landmark, grouped by
its Chart), `library/<ChartID>/_schema.json` (one per Chart directory),
`terrain.jsonl`, and `marks.jsonl`. The directory names `assets/` and
`beacons/` are reserved and SHALL NOT be emitted.

No database-local identifier — rowid, sequence, or similar — SHALL appear
anywhere in a Bundle. Every identifier is a domain-level stable handle
(`MarkID`, `ChartID`, `FeatureID`, `GlyphID`) serialized transparently.

#### Scenario: Landmark placement

- **WHEN** a store containing a Landmark of Chart `c4d8` with root Mark id
  `018f…` is exported
- **THEN** the Bundle contains `library/c4d8/018f….md` and
  `library/c4d8/_schema.json`

#### Scenario: No reserved directories

- **WHEN** any store is exported
- **THEN** the Bundle contains no `assets/` and no `beacons/` directory

### Requirement: Export-Bundles carry declared identity and never source keys

Every identity in an export-Bundle SHALL be a declared `id`. The `src` form
(importer provenance) SHALL NOT appear in an export-Bundle; an ingester
encountering a `src`-keyed identity in Bundle position SHALL refuse it as
malformed rather than resolving it.

#### Scenario: A src-keyed file is refused

- **WHEN** a Bundle file carries an identity comment of the form
  `<!-- wp:mark {"src": …} -->`
- **THEN** ingest refuses with a malformed-Bundle error naming the file and
  line
- **AND** the store is unchanged

### Requirement: The Landmark file serializes the flattened root Mark as frontmatter

A Landmark file SHALL open with YAML frontmatter carrying the flattened root
Mark: `title`, `chart` (a `ChartID` — 4-character base-36 or `builtin.{name}`),
`created` and `updated` as ISO-8601 UTC wall-clock timestamps with explicit
`Z` and millisecond precision, and `readings` keyed by `FeatureID`.

Human-readable feature names SHALL ride only as YAML comments — advisory,
regenerated on every export, and ignored on ingest. Relation Readings
serialize as lists of `{ref, label?}` where `ref` is normative and `label` is
advisory and regenerated. Rich-text Readings serialize as YAML block scalars
with normalized chomping. No hybrid logical clock, op-log record, or sync
artifact SHALL appear anywhere in a Bundle.

#### Scenario: Reading keys are FeatureIDs

- **WHEN** a Landmark with a Reading on feature `a1b2` ("Attendees") is
  exported
- **THEN** the frontmatter key is `a1b2`, with `# Attendees` present only as
  a comment

#### Scenario: Advisory labels are not trusted

- **WHEN** a Bundle is ingested whose reading comments and `label` values
  were hand-edited to arbitrary text
- **THEN** the ingested store is identical to ingesting the same Bundle with
  those comments and labels removed

#### Scenario: Timestamps survive ingest

- **WHEN** a Bundle whose frontmatter carries `updated: 2026-08-02T09:41:00.000Z`
  is ingested
- **THEN** the resulting Mark's update timestamp is exactly that instant

### Requirement: Every body Mark carries an identity comment

Every Mark in a Landmark body SHALL be preceded by a `wp:mark` identity
comment carrying its `id`. Container Marks SHALL use paired
`<!-- wp:mark … --> … <!-- /wp:mark -->` comments; leaf Marks SHALL use a
single unpaired comment. Unknown fields inside an identity comment SHALL be
preserved through re-serialization.

A Mark shared by multiple Landmarks SHALL be rendered once and byte-copied
into every containing file — identical identity comment, identical content —
so divergence within a single export is impossible. On ingest, identical
`id` resolves to the same Mark.

#### Scenario: Leaf and container pairing

- **WHEN** a Landmark body containing a paragraph and a blockquote with
  children is exported
- **THEN** the paragraph carries one preceding identity comment and no
  closing comment
- **AND** the blockquote carries an opening and a closing comment enclosing
  its children

#### Scenario: Unknown comment fields round-trip

- **WHEN** a Bundle whose identity comment carries an unrecognized field is
  ingested and re-exported
- **THEN** the field is present, unmodified, in the re-exported comment

### Requirement: Divergent same-id duplicates refuse loudly

When ingest encounters the same `id` with byte-different content across
files, it SHALL refuse the Bundle with a per-UUID conflict report naming
every involved file. It SHALL NOT pick a winner silently.

#### Scenario: Bundle surgery is detected

- **WHEN** two hand-merged files carry the same `wp:mark` id with different
  paragraph text
- **THEN** ingest refuses with a conflict report listing that id and both
  file paths
- **AND** the store is unchanged

### Requirement: Terrain edges export by exactly one of three tiers

Structural (child) edges SHALL be carried by document nesting; Reading-bound
relational edges by `readings.*[].ref` in frontmatter; every other edge kind
by a `terrain.jsonl` line. The exporter's dispatch over edge kinds SHALL be
exhaustive without a wildcard: introducing a new edge kind without an explicit
tier assignment fails compilation. The ingester SHALL reconstruct Terrain as
exactly the union of the three tiers — no edge is silently dropped.

`terrain.jsonl` lines are `{src, dst, kind, order?, extra?}`. Constitutive
lines (tier-three kinds) are the edge record. Annotative lines (tier-one/two
kinds) exist only to attach `extra` to an edge whose structure lives in the
projection, addressed by the natural key `(src, dst, kind, order)`; an
annotative line matching no projected edge SHALL refuse the Bundle as
malformed.

A `TerrainID` SHALL NOT appear in a Bundle; edge identities regenerate
unconditionally on ingest. `order` is an ordinal among parallel
`(src, dst, kind)` edges; runtime order strings regenerate on ingest and
SHALL NOT be carried.

#### Scenario: Custom edge survives via the sidecar

- **WHEN** a store containing a custom-kind edge with an `extra` payload is
  exported and re-ingested
- **THEN** the edge exists with its kind, endpoints, and `extra` intact, under
  a fresh edge identity

#### Scenario: Orphan annotative line refuses

- **WHEN** `terrain.jsonl` carries a child-kind line whose
  `(src, dst, kind, order)` matches no edge reconstructible from the
  projection
- **THEN** ingest refuses with a malformed-Bundle error citing that line

#### Scenario: New edge kind cannot ship untiered

- **WHEN** a new `TerrainKind` variant is added without a tier assignment in
  the exporter dispatch
- **THEN** the workspace fails to compile

### Requirement: Entity residue lands in per-entity carriers

`extra` on Marks (Landmark roots and Content Marks alike) SHALL export to
`marks.jsonl` as one `{id, extra}` line per extra-bearing Mark. `extra` on
edges rides in their `terrain.jsonl` lines. `extra` on Charts rides inside
the Chart's `_schema.json`. Re-ingesting a Bundle SHALL restore every `extra`
value unchanged.

#### Scenario: Mark extra round-trips

- **WHEN** a store whose Content Mark carries namespaced `extra` is exported
  and re-ingested
- **THEN** that Mark carries the identical `extra` value

### Requirement: Every referenced Chart exports a valid, lossless schema

Every Chart referenced by an exported Landmark — builtin or custom — SHALL
emit a `_schema.json` that validates against the JSON Schema Draft-07
meta-schema and carries the Chart losslessly (features, name, description,
icon, and the Chart's `extra`). The ingester SHALL validate each Landmark's
frontmatter readings against its Chart's schema.

On ingest, `builtin.*` bindings resolve to the application's own builtin
Charts — a Bundle SHALL NOT overwrite a builtin definition; a Bundle builtin
schema requiring what the application's builtin lacks produces a per-Chart
conflict entry. Custom Charts ingest as upserts from their schema documents.

#### Scenario: Meta-schema validity

- **WHEN** any store is exported
- **THEN** every `_schema.json` in the Bundle validates against the Draft-07
  meta-schema

#### Scenario: Custom Chart round-trips

- **WHEN** a store with a user-defined Chart (select features with glyphs,
  a reference feature, and Chart `extra`) is exported and ingested into an
  empty store
- **THEN** the resulting Chart equals the original, id included

#### Scenario: Invalid reading refuses

- **WHEN** a Landmark file carries a reading value that violates its Chart's
  schema
- **THEN** ingest reports a validation error naming the file and feature

### Requirement: Ingesting an export reproduces the store

For any Waypoints store, `ingest(export(store))` SHALL equal the store over
content, structure, and identity: every live Mark (data, `extra`, created and
updated timestamps, id), every live Terrain edge (endpoints, kind, ordinal
sequence, `extra`), and every Chart. Edge identities and order strings are
regenerated machinery and are exempt. Soft-deleted Marks and edges SHALL NOT
export; a Bundle is a freeze-frame of live knowledge, and deletion in a
freeze-frame is absence.

#### Scenario: Roundtrip equality

- **WHEN** a store is exported and the Bundle is ingested into an empty store
- **THEN** the two stores are equal over content, structure, and identity,
  with fresh edge identities and order strings

#### Scenario: Tombstones stay behind

- **WHEN** a store containing a soft-deleted Mark is exported
- **THEN** no file, sidecar line, or schema in the Bundle references that
  Mark's id

### Requirement: The projection reads as text once comments are stripped

Stripping every comment from every projection file SHALL yield documents
that read as text, proper — every remaining marker a legible dialect
convention, with zero machine-shaped residue (`wp:` markers and UUIDs
appearing only as link targets). Renderer conformance is not the bar; the
claim is Org-Mode-style plain-text legibility. Content a reader should see
rides in dialect syntax or attributes; identity and provenance ride in
comments; nothing else exists.

#### Scenario: The 1980 test

- **WHEN** all comments are stripped from every `.md` file in an exported
  Bundle
- **THEN** no file contains a `wp:` marker, a UUID, or other machine-shaped
  residue outside link targets
- **AND** every file still parses under the dialect grammar

### Requirement: Inline references project as wiki links

An inline mention SHALL project as a `[[…]]` wiki link whose interior is a
bare `MarkID`, identifying the mentioned Mark losslessly. A label after a pipe
(`[[target|label]]`) is the anchor text and SHALL survive export and ingest
unchanged; only a mention carrying no label SHALL have one generated, from the
target's title, on ingest. Ingest reconstructs the inline reference mark with
the same target id.

The interior is a bare id rather than a relative Landmark path because a
mention may target a Content Mark, which has no Landmark file, and because a
path would couple body bytes to export layout.

#### Scenario: Landmark mention round-trips

- **WHEN** a paragraph mentioning a Landmark is exported
- **THEN** the projected text carries a `[[…]]` wiki link resolving to that
  Landmark's `MarkID`
- **AND** re-ingesting reconstructs the mention with the same target id

#### Scenario: Anchor text is content

- **WHEN** a mention whose anchor text differs from its target's title is
  exported and re-ingested
- **THEN** the anchor text is unchanged

#### Scenario: A bare mention gains a label

- **WHEN** a Bundle carrying `[[<MarkID>]]` with no label is ingested
- **THEN** the reconstructed mention's text is the target's title

### Requirement: Export output is byte-stable

Exporting the same store twice SHALL produce byte-identical Bundles: stable
file ordering, stable key ordering (unordered collections sorted at the
serialization boundary), and normalized YAML chomping.

#### Scenario: Determinism

- **WHEN** one store is exported twice
- **THEN** the two Bundles are byte-identical file by file

### Requirement: The manifest versions the Bundle and gates ingest

`manifest.json` SHALL carry `bundle_format` (semver), exporter application
version, export timestamp, entity counts, and optional territory identity.
An ingester SHALL refuse a Bundle whose `bundle_format` it does not support,
before reading any other file. While the format is pre-freeze, exported
Bundles carry a `0.x` version; the frozen `1.0.0` is reserved for the data
covenant.

#### Scenario: Newer Bundle refused

- **WHEN** a Bundle whose manifest declares an unsupported `bundle_format` is
  ingested
- **THEN** ingest refuses with a version error and reads nothing else

### Requirement: Ingest is atomic

A refused Bundle SHALL leave the store untouched: ingest applies as a single
transaction, and every refusal path — malformed grammar, identity conflict,
schema violation, orphan annotative line, version gate — rolls back
completely.

#### Scenario: No partial ingest

- **WHEN** a Bundle's final file carries a divergent duplicate and every
  earlier file is valid
- **THEN** after the refusal, the store contains none of the Bundle's
  entities

## Decisions

### 2026-08-18 — Soft-deleted entities are absent from Bundles

_Rationale:_ a Bundle is a freeze-frame carrying knowledge, not an op log.
Exporting tombstones would either resurrect deleted content on ingest or
demand marker files that violate legibility; deletion history is sync's
cargo, excluded from Bundles by the freeze-frame ruling.
_Rejected:_ exporting tombstoned entities with a deletion marker — corpse
files a CommonMark reader must see, and op-log semantics smuggled into a
format that rules them out.

### 2026-08-18 — Chart losslessness rides `x-waypoints` inside a real Draft-07 document

_Rationale:_ Draft-07 keywords generated from features make the schema usable
by external tooling and by ingest validation, but cannot carry feature-kind
richness (display modes, glyph palettes, grouped selects); fidelity demands
the canonical Chart travel too. Unknown keywords are Draft-07-legal, so one
reserved `x-waypoints` member carries it — and the Chart's `extra` with it —
while the document still validates against the meta-schema.
_Rejected:_ Draft-07-only (silently lossy — a fidelity violation by
construction); a separate chart sidecar (the schema file is already the
per-Chart structured carrier).

### 2026-10-20 — A mention's anchor text is content, not an advisory label

_Rationale:_ the label after the pipe is what a person chose to call the target
in that sentence. Regenerating it from the target's title on every export would
discard writing nobody asked to lose — "the Foundation series" is not a worse
name for a book than the book's title. Only a bare `[[id]]` leaves nothing to
preserve, and that is the one case a label is generated, on ingest, by the side
that holds the store. Frontmatter's relation labels are unaffected and remain
advisory: `Reading::Relation` holds refs and has nowhere for a label to land.
_Rejected:_ regenerating always (the earlier draft of this requirement — silent
content loss on a round trip that exists to be lossless).

### 2026-10-20 — The wiki-link interior is a bare `MarkID`

_Rationale:_ a relative Landmark path cannot express every mention the model
allows — a mention may target a Content Mark, which has no Landmark file to
point at — and embedding the Chart directory in body text would couple content
bytes to export layout, so re-grouping a Chart would rewrite every mentioning
file. The id resolves trivially for Waypoints and stays a unique greppable
token for anything else.
_Rejected:_ a relative Landmark path; a title-keyed interior (not lossless —
titles change and collide).

### 2026-08-18 — Inline mentions are wiki links, not comments

_Rationale:_ a mention's target is content semantics — a reader should see a
link — so the carrier rule assigns it to syntax, not comments. The dialect's
internal-reference form is the `[[…]]` wiki link (John's ruling,
2026-08-18); a mention needing extra metadata (non-default reference display
mode) may escalate to an inline component, the only reserved use of inline
components.
_Rejected:_ raw `@uuid` text (illegible as text); per-mention identity
comments (identity comments carry Mark identity, and inline comment pairs
would shred readability); `[label](wp:<MarkID>)` / relative-path CommonMark
links (superseded by the wiki-link ruling — internal and external references
should not share a spelling).

### 2026-08-18 — Pre-freeze Bundles declare `0.x`, never `1.0.0`

_Rationale:_ the covenant binds `1.0.0` to the frozen format; a draft
exporter stamping `1.0.0` would mint Bundles the covenant must honor before
it exists. Version-gating at the manifest keeps refusal cheap and total.
_Rejected:_ stamping `1.0.0-draft.N` pre-release tags — semver orders them
before `1.0.0`, but "looks frozen at a glance" is exactly the confusion a
plain `0.x` avoids.
