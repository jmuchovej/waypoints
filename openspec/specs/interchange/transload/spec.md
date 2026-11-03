# Transload

## Purpose

The Bundle format: a store exported as a directory of plain markdown, YAML
frontmatter and JSON sidecars that reads as text once comments are stripped,
and ingests back into an equal store — or is refused whole. And the Fragment
grammar: what an importer plugin emits, the data model's entities keyed by
source key wherever the core would mint an identifier.

## Requirements

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
`icon` when the Landmark has one, `created` and `updated` as ISO-8601 UTC
wall-clock timestamps with explicit `Z` and millisecond precision, and
`readings` keyed by `FeatureID`.

The `chart` value is the whole `ChartID`, `builtin.` prefix included — not the
display name, which would collapse a builtin binding and a custom Chart of the
same name onto one identity.

#### Scenario: An icon survives

- **WHEN** a Landmark carrying an icon is exported and re-ingested
- **THEN** the icon is unchanged

The frontmatter `created` and `updated` SHALL be marked `# advisory`:
`marks.jsonl` is normative for every Mark's timestamps, and the frontmatter
values are read only when a Landmark has no `marks.jsonl` line.

Human-readable feature names SHALL ride only as YAML comments trailing the
reading's key line — advisory, regenerated on every export, and ignored on
ingest — so a reading costs no extra line. Relation Readings
serialize as lists of `{ref, label?}` where `ref` is normative and `label` is
advisory and regenerated. Rich-text Readings serialize as YAML block scalars
with normalized chomping. No hybrid logical clock, op-log record, or sync
artifact SHALL appear anywhere in a Bundle.

#### Scenario: Reading keys are FeatureIDs

- **WHEN** a Landmark with a Reading on feature `a1b2` ("Attendees") is
  exported
- **THEN** the frontmatter key is `a1b2`, with `# Attendees` present only as
  a comment on that key's line

#### Scenario: Advisory labels are not trusted

- **WHEN** a Bundle is ingested whose reading comments and `label` values
  were hand-edited to arbitrary text
- **THEN** the ingested store is identical to ingesting the same Bundle with
  those comments and labels removed

#### Scenario: Timestamps survive ingest

- **WHEN** a Bundle whose `marks.jsonl` line for a Landmark carries
  `"updated":"2026-08-02T09:41:00.000Z"` is ingested
- **THEN** the resulting Mark's update timestamp is exactly that instant,
  whatever its frontmatter's advisory `updated` says

### Requirement: Every body Mark carries an identity comment

Every Mark in a Landmark body SHALL be preceded by a `wp:mark` identity
comment carrying its `id` and nothing else: `<!-- wp:mark <MarkID> -->`. A
Mark with children SHALL use paired `<!-- wp:mark … --> … <!-- /wp:mark -->`
comments enclosing them; a Mark without children SHALL use a single unpaired
comment. Sibling blocks SHALL be separated by a blank line, so each reads as
its own paragraph to any markdown reader and an editor can fold it.

A Landmark nested under another Mark SHALL appear in its parent's body at its
sibling position, as an identity comment over a `[[Title]]` line naming it;
its content lives in its own file.

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

#### Scenario: Siblings are separated

- **WHEN** a Landmark body with two sibling paragraphs is exported
- **THEN** a blank line stands between the first paragraph and the second's
  identity comment

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

### Requirement: Terrain edges export by exactly one of two tiers

Structural (child) edges SHALL be carried by document nesting; every other edge
kind, `Reading` included, by a `terrain.jsonl` line. The exporter's dispatch
over edge kinds SHALL be exhaustive without a wildcard: introducing a new edge
kind without an explicit tier assignment fails compilation. The ingester SHALL
reconstruct Terrain as exactly the union of the two tiers — no edge is silently
dropped, and none is synthesized from reading refs.

`terrain.jsonl` lines are `{src, dst, kind, order?, extra?}`. Constitutive
lines are the edge record. Annotative lines (child kind) exist only to attach
`extra` to an edge whose structure lives in the projection, addressed by the
natural key `(src, dst, kind, order)`; an annotative line matching no projected
edge SHALL refuse the Bundle as malformed.

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

Every Mark in the Bundle (Landmark roots and Content Marks alike) SHALL export
one `marks.jsonl` line, `{id, created, updated, extra?}`, carrying its
normative timestamps and, when it has one, its `extra`. A Content Mark with no
line SHALL refuse. `extra` on edges rides in their `terrain.jsonl` lines. `extra` on Charts rides inside
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
convention, with zero machine-shaped residue (`wp:` markers, and UUIDs save
as the target of a link to a Mark the Bundle does not carry). Renderer conformance is not the bar; the
claim is Org-Mode-style plain-text legibility. Content a reader should see
rides in dialect syntax or attributes; identity and provenance ride in
comments; nothing else exists.

#### Scenario: The 1980 test

- **WHEN** all comments are stripped from every `.md` file in an exported
  Bundle
- **THEN** no file contains a `wp:` marker, a UUID, or other machine-shaped
  residue, save the bare target of a link to a Mark the Bundle does not carry
- **AND** every file still parses under the dialect grammar

### Requirement: Inline references project as wiki links

An inline mention SHALL project as a `[[…]]` wiki link naming its target in
plain text, and ingest SHALL resolve the name back to the same `MarkID`:

- A Landmark is named by its title's plain text. Every Landmark sharing a
  title — and any whose title is empty or reads as an id — SHALL carry a
  suffix, `Title (4f2a)`, from the start of its id, lengthened until the
  holders differ. No holder is left unsuffixed.
- A Content Mark is named `[[Page Title#4f2a]]`: the Landmark whose file
  writes it, plus the shortest id prefix (four characters or more) that is
  unique among that file's blocks. A block written in several files is named
  in the first by Landmark id.
- A target the Bundle does not carry keeps its bare `MarkID`.

A label after a pipe (`[[target|label]]`) is the anchor text and SHALL survive
export and ingest unchanged. A label equal to its Landmark target's title is
elided, except inside a Landmark title, whose names are built before any
mention resolves. A mention carrying no label SHALL have one generated, from
the target's title, on ingest.

#### Scenario: Landmark mention round-trips

- **WHEN** a paragraph mentioning a Landmark titled "Cosmos" is exported
- **THEN** the projected text carries `[[Cosmos]]`
- **AND** re-ingesting reconstructs the mention with the same target id

#### Scenario: Shared titles are all suffixed

- **WHEN** two Landmarks titled "Cosmos" are mentioned and exported
- **THEN** both links carry a suffix drawn from their ids
- **AND** each resolves to its own Landmark on ingest

#### Scenario: A block mention names its page

- **WHEN** a paragraph mentioning a Content Mark is exported
- **THEN** the link reads `[[Page Title#prefix]]`
- **AND** re-ingesting reconstructs the mention with that block's id

#### Scenario: Anchor text is content

- **WHEN** a mention whose anchor text differs from its target's title is
  exported and re-ingested
- **THEN** the anchor text is unchanged

#### Scenario: A bare mention gains a label

- **WHEN** a Bundle carrying `[[Title]]` with no label is ingested
- **THEN** the reconstructed mention's text is the target's title

### Requirement: Export output is byte-stable

Exporting the same store twice SHALL produce byte-identical Bundles: stable
file ordering, stable key ordering (unordered collections sorted at the
serialization boundary), and normalized YAML chomping.

#### Scenario: Determinism

- **WHEN** one store is exported twice
- **THEN** the two Bundles are byte-identical file by file

### Requirement: The manifest versions the Bundle and gates ingest

`manifest.json` SHALL carry `bundle_format` (semver), the exporting
application's version, the `domain` crate's version, the export timestamp,
entity counts, a census of the content variants the Bundle uses, and optional
territory identity. While the format is pre-freeze, exported Bundles carry a
`0.x` version; the frozen `1.0.0` is reserved for the data covenant.

An ingester SHALL support a _set_ of `bundle_format` versions, not only the one
it writes, and SHALL refuse a Bundle outside that set before reading any other
file. The supported set narrows only on a deliberate schedule, so a Bundle
written by one build stays readable by later ones for an extended period.

The `domain` version answers whether a Bundle's grammar may be newer than the
reading build's: `domain` is its own release group, so adding a content,
reading, feature, or terrain variant necessarily bumps it. The census answers
whether anything new is actually _present_ — a Bundle written by a newer
`domain` that uses no new variants is fully readable, and the version alone
cannot say so.

#### Scenario: An unsupported format is refused before anything is read

- **WHEN** a Bundle whose manifest declares a `bundle_format` outside the
  ingester's supported set is ingested
- **THEN** ingest refuses with a version error and reads nothing else

#### Scenario: An older supported format still ingests

- **WHEN** a Bundle declaring an earlier `bundle_format` that is still within
  the supported set is ingested
- **THEN** ingest proceeds

#### Scenario: A newer grammar is reported, not guessed at

- **WHEN** a Bundle declares a `domain` version newer than the reading build's
- **THEN** the result reports which version wrote it, alongside the census of
  variants it uses

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

### Requirement: The Bundle grammar is usable without the persistence engine or a source of entropy

The Bundle grammar — layout and manifest, frontmatter, identity comments,
sidecar lines, the Chart schema document, link names, and the refusal
vocabulary — SHALL be usable by a consumer that links no database engine and
no source of entropy, including a consumer compiled for a WebAssembly target.
Export, ingest, and the validation of readings against a Chart schema, which
need a store or a schema validator, SHALL be separable from the grammar they
use, and SHALL remain present by default. A Bundle directory name SHALL spell
a Chart id exactly as the Bundle carries it, prefix included.

#### Scenario: The grammar compiles for WebAssembly without an engine or entropy

- **WHEN** the Bundle crate is checked for `wasm32-unknown-unknown` with
  export and ingest excluded
- **THEN** it compiles
- **AND** its dependency graph contains no database engine, no schema
  validator, no system-randomness crate, and no JavaScript-host binding

#### Scenario: A module built from the grammar imports nothing

- **WHEN** a module that links the grammar and the data model is built for
  `wasm32-unknown-unknown`
- **THEN** its import section is empty

#### Scenario: Export and ingest are unchanged by default

- **WHEN** the application is built as before
- **THEN** export and ingest are present
- **AND** the committed getting-started Bundle round-trips exactly as before

#### Scenario: A builtin Chart keeps its prefix in its directory name

- **WHEN** a Landmark on the builtin page Chart is exported
- **THEN** its file lives under `library/builtin.page/`

### Requirement: Fragments carry source-keyed identity and never declared ids

A Fragment — what an importer emits — SHALL identify every entity it defines
by a source key: a non-empty string the importer chooses, stable across runs
of the same source, scoped by the importer's namespace, and opaque to
Waypoints. No `MarkID`, `TerrainID`, `ChartID`, `FeatureID`, or `GlyphID`
SHALL appear in a Fragment; identifiers are minted by the core on ingest,
never by an importer, and a Fragment has no way to name one. A Fragment entity
SHALL ignore fields it does not define, except that a field named `id` SHALL
be refused.

#### Scenario: A Landmark Fragment is keyed, not identified

- **WHEN** an importer emits a Landmark for a source page
- **THEN** the Landmark carries `src` and no `id`
- **AND** every Content Fragment beneath it carries `src` and no `id`

#### Scenario: An `id` field is refused while other unknown fields are ignored

- **WHEN** a Content Fragment carries a field named `id`
- **THEN** deserialization refuses it as not a Fragment
- **AND** the same Fragment with that field renamed to one the grammar does
  not define deserializes, ignoring the field

### Requirement: Fragment entities mirror the data model's, with keys in every minted slot

A Fragment SHALL express a Landmark with its title, icon, protochart binding,
readings, timestamps, and `extra`; a tree of Content beneath it whose nesting
carries the child edges; and Terrain lines — with a source key in every slot
the data model fills with a minted identifier, and the same value otherwise.
Each Fragment entity is named after the data-model entity it mirrors, so the
vocabulary maps one for one: a Landmark Fragment, a Content Fragment, a
Terrain Fragment, and the reading and feature-kind mirrors beneath them. Block
content, rich text, icons, and scalar readings are the data model's own types,
unchanged. Select options and groups are mirrored without their identifiers
and keyed by source key.

#### Scenario: Nesting is the child edge

- **WHEN** a Content Fragment carries children
- **THEN** each child is a Content Fragment beneath it in order
- **AND** no child-kind Terrain line is needed to express that containment

#### Scenario: A select reading is keyed

- **WHEN** an importer emits a Landmark whose reading selects an option of a
  protochart feature
- **THEN** the reading names the feature by its key and the option by its key
- **AND** no glyph identifier appears

### Requirement: A Fragment binds a Landmark to a protochart or to nothing, never to a Chart

An importer SHALL describe the structure it discovers in a source as a
protochart: a source-keyed entity carrying a display name, description, and
icon for a person to recognize it by, and features keyed by source key, each
with a name, description, requiredness, and kind. A Landmark Fragment SHALL
bind either to a protochart by key or to no protochart. A Landmark bound to no
protochart SHALL carry no readings. A Landmark bound to a protochart SHALL key
its readings by that protochart's feature keys and its select values by the
feature's option or group keys. A reference feature's allowed types, when
given, SHALL name protocharts by key. A Fragment SHALL have no way to name a
Chart, builtin or otherwise; resolving protocharts to Charts is not the
Fragment's concern.

#### Scenario: A plain page binds to nothing

- **WHEN** the fixture importer emits a Landmark for a markdown file
- **THEN** the Landmark has no protochart binding and no readings

#### Scenario: A discovered structure is a protochart

- **WHEN** an importer emits a protochart with a select feature and a Landmark
  bound to it with that feature set
- **THEN** the protochart's feature and option are keyed by source key
- **AND** the Landmark's reading names the feature and option by those keys
- **AND** nothing in either names a Chart

### Requirement: Keys have four scopes under the importer's namespace

Source keys SHALL be scoped as follows, all within the importer's namespace:
Landmark and Content Fragments share one key space; protocharts have their
own; feature keys are scoped by the protochart that defines them; option and
group keys are scoped by the feature that defines them. Which space a string
is read in is determined by where it appears, never by its spelling.

#### Scenario: A feature key may repeat across protocharts

- **WHEN** two protocharts each define a feature keyed `status`
- **THEN** both Fragments are valid
- **AND** a Landmark's `status` reading is read against its own protochart

### Requirement: References name source keys, including ones not yet defined

A reference inside a Fragment — a mention in rich text, a reference reading,
either end of a Terrain line, a Landmark's protochart binding — SHALL name its
target by source key. A mention carries the key in the same slot a Bundle
fills with the target's id. A Fragment MAY reference a Mark key that no entity
in the same output or run defines, and a Landmark MAY bind to a protochart key
that a later output of the same run defines; resolving Mark keys across
outputs and runs is not the Fragment's concern.

#### Scenario: A mention targets a key

- **WHEN** an importer emits a paragraph mentioning another source page
- **THEN** the mention's data is that page's source key
- **AND** the paragraph is otherwise an ordinary content block

#### Scenario: A forward reference is a valid Fragment

- **WHEN** an entry references a page the importer has not yet seen
- **THEN** the entry's Fragment carries the reference by key
- **AND** the Fragment is valid on its own

### Requirement: A Fragment carries nothing the core owns

A Fragment SHALL NOT carry edge identities, order strings, content hashes,
clocks, sync artifacts, or reading-kind Terrain lines. A reference reading is
carried as a reading only; whether it is also indexed as a Terrain edge is the
core's decision, never the importer's. Terrain lines in a Fragment are of kind
child (annotative, to attach `extra` to a nested edge), reference, or custom.
Timestamps are optional per entity and, when present, are ISO-8601 UTC
wall-clock values with explicit `Z`. `extra` is optional per entity and, when
present, uses namespaced keys as the data model requires. Order among siblings
and among parallel Terrain lines is positional or an ordinal, never a runtime
key.

#### Scenario: Timestamps are optional

- **WHEN** an importer emits a Content Fragment with no timestamps
- **THEN** the Fragment is valid
- **AND** one carrying `created` and `updated` is equally valid

#### Scenario: A reference reading is only a reading

- **WHEN** an importer emits a Landmark with a reference reading
- **THEN** the Fragment carries the reading and no reading-kind Terrain line
- **AND** the Fragment grammar offers no reading-kind Terrain line to emit

### Requirement: Fragment serialization is canonical, and a run's output is byte-stable

The SDK SHALL serialize Fragment output in canonical form: stable field order,
ordered maps in the importer's order, unordered collections sorted, timestamps
at millisecond precision. Deserializing an output the SDK wrote and
serializing it again SHALL reproduce it byte for byte. A run over the same
configuration and the same entry sequence SHALL produce byte-identical output.

#### Scenario: Round-trip is the identity

- **WHEN** an SDK-written Fragment output is deserialized and serialized again
- **THEN** the bytes are identical

#### Scenario: Same run, same bytes

- **WHEN** the fixture importer is run twice over the same fixtures with the
  same configuration
- **THEN** every output is byte-identical between the runs

### Requirement: The Fragment envelope is versioned on two axes

A Fragment output SHALL be versioned by `fragment_format` (semver; additive
minor, breaking major; `0.x` while pre-freeze) and by the `domain` grammar
version it was emitted against, both stamped by the SDK. A completed run's
final output SHALL carry a census of the content, reading, feature-kind, and
terrain-kind variants the run actually emitted, computed from the output.

#### Scenario: The census is computed

- **WHEN** a run emits paragraphs and headings and nothing else
- **THEN** its final census lists exactly those content variants

#### Scenario: The envelope version is the SDK's

- **WHEN** a plugin's output is read
- **THEN** its `fragment_format` is the version the SDK it was built with
  emits
