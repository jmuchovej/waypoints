## ADDED Requirements

### Requirement: Fragments carry source-keyed identity and never declared ids

A Fragment — what an importer emits — SHALL identify every entity it defines by a source key: a non-empty string the importer chooses, stable across runs of the same source, scoped by the importer's namespace, and opaque to Waypoints.
No `MarkID`, `TerrainID`, `ChartID`, `FeatureID`, or `GlyphID` SHALL appear in a Fragment; identifiers are minted by the core on ingest, never by an importer, and a Fragment has no way to name one.
A Fragment entity SHALL ignore fields it does not define, except that a field named `id` SHALL be refused.

#### Scenario: A Landmark Fragment is keyed, not identified

- **WHEN** an importer emits a Landmark for a source page
- **THEN** the Landmark carries `src` and no `id`
- **AND** every Content Fragment beneath it carries `src` and no `id`

#### Scenario: An `id` field is refused while other unknown fields are ignored

- **WHEN** a Content Fragment carries a field named `id`
- **THEN** deserialization refuses it as not a Fragment
- **AND** the same Fragment with that field renamed to one the grammar does not define deserializes, ignoring the field

### Requirement: Fragment entities mirror the data model's, with keys in every minted slot

A Fragment SHALL express a Landmark with its title, icon, protochart binding, readings, timestamps, and `extra`; a tree of Content beneath it whose nesting carries the child edges; and Terrain lines — with a source key in every slot the data model fills with a minted identifier, and the same value otherwise.
Each Fragment entity is named after the data-model entity it mirrors, so the vocabulary maps one for one: a Landmark Fragment, a Content Fragment, a Terrain Fragment, and the reading and feature-kind mirrors beneath them.
Block content, rich text, icons, and scalar readings are the data model's own types, unchanged.
Select options and groups are mirrored without their identifiers and keyed by source key.

#### Scenario: Nesting is the child edge

- **WHEN** a Content Fragment carries children
- **THEN** each child is a Content Fragment beneath it in order
- **AND** no child-kind Terrain line is needed to express that containment

#### Scenario: A select reading is keyed

- **WHEN** an importer emits a Landmark whose reading selects an option of a protochart feature
- **THEN** the reading names the feature by its key and the option by its key
- **AND** no glyph identifier appears

### Requirement: A Fragment binds a Landmark to a protochart or to nothing, never to a Chart

An importer SHALL describe the structure it discovers in a source as a protochart: a source-keyed entity carrying a display name, description, and icon for a person to recognize it by, and features keyed by source key, each with a name, description, requiredness, and kind.
A Landmark Fragment SHALL bind either to a protochart by key or to no protochart.
A Landmark bound to no protochart SHALL carry no readings.
A Landmark bound to a protochart SHALL key its readings by that protochart's feature keys and its select values by the feature's option or group keys.
A reference feature's allowed types, when given, SHALL name protocharts by key.
A Fragment SHALL have no way to name a Chart, builtin or otherwise; resolving protocharts to Charts is not the Fragment's concern.

#### Scenario: A plain page binds to nothing

- **WHEN** the fixture importer emits a Landmark for a markdown file
- **THEN** the Landmark has no protochart binding and no readings

#### Scenario: A discovered structure is a protochart

- **WHEN** an importer emits a protochart with a select feature and a Landmark bound to it with that feature set
- **THEN** the protochart's feature and option are keyed by source key
- **AND** the Landmark's reading names the feature and option by those keys
- **AND** nothing in either names a Chart

### Requirement: Keys have four scopes under the importer's namespace

Source keys SHALL be scoped as follows, all within the importer's namespace: Landmark and Content Fragments share one key space; protocharts have their own; feature keys are scoped by the protochart that defines them; option and group keys are scoped by the feature that defines them.
Which space a string is read in is determined by where it appears, never by its spelling.

#### Scenario: A feature key may repeat across protocharts

- **WHEN** two protocharts each define a feature keyed `status`
- **THEN** both Fragments are valid
- **AND** a Landmark's `status` reading is read against its own protochart

### Requirement: References name source keys, including ones not yet defined

A reference inside a Fragment — a mention in rich text, a reference reading, either end of a Terrain line, a Landmark's protochart binding — SHALL name its target by source key.
A mention carries the key in the same slot a Bundle fills with the target's id.
A Fragment MAY reference a Mark key that no entity in the same output or run defines, and a Landmark MAY bind to a protochart key that a later output of the same run defines; resolving Mark keys across outputs and runs is not the Fragment's concern.

#### Scenario: A mention targets a key

- **WHEN** an importer emits a paragraph mentioning another source page
- **THEN** the mention's data is that page's source key
- **AND** the paragraph is otherwise an ordinary content block

#### Scenario: A forward reference is a valid Fragment

- **WHEN** an entry references a page the importer has not yet seen
- **THEN** the entry's Fragment carries the reference by key
- **AND** the Fragment is valid on its own

### Requirement: A Fragment carries nothing the core owns

A Fragment SHALL NOT carry edge identities, order strings, content hashes, clocks, sync artifacts, or reading-kind Terrain lines.
A reference reading is carried as a reading only; whether it is also indexed as a Terrain edge is the core's decision, never the importer's.
Terrain lines in a Fragment are of kind child (annotative, to attach `extra` to a nested edge), reference, or custom.
Timestamps are optional per entity and, when present, are ISO-8601 UTC wall-clock values with explicit `Z`.
`extra` is optional per entity and, when present, uses namespaced keys as the data model requires.
Order among siblings and among parallel Terrain lines is positional or an ordinal, never a runtime key.

#### Scenario: Timestamps are optional

- **WHEN** an importer emits a Content Fragment with no timestamps
- **THEN** the Fragment is valid
- **AND** one carrying `created` and `updated` is equally valid

#### Scenario: A reference reading is only a reading

- **WHEN** an importer emits a Landmark with a reference reading
- **THEN** the Fragment carries the reading and no reading-kind Terrain line
- **AND** the Fragment grammar offers no reading-kind Terrain line to emit

### Requirement: Fragment serialization is canonical, and a run's output is byte-stable

The SDK SHALL serialize Fragment output in canonical form: stable field order, ordered maps in the importer's order, unordered collections sorted, timestamps at millisecond precision.
Deserializing an output the SDK wrote and serializing it again SHALL reproduce it byte for byte.
A run over the same configuration and the same entry sequence SHALL produce byte-identical output.

#### Scenario: Round-trip is the identity

- **WHEN** an SDK-written Fragment output is deserialized and serialized again
- **THEN** the bytes are identical

#### Scenario: Same run, same bytes

- **WHEN** the fixture importer is run twice over the same fixtures with the same configuration
- **THEN** every output is byte-identical between the runs

### Requirement: The Fragment envelope is versioned on two axes

A Fragment output SHALL be versioned by `fragment_format` (semver; additive minor, breaking major; `0.x` while pre-freeze) and by the `domain` grammar version it was emitted against, both stamped by the SDK.
A completed run's final output SHALL carry a census of the content, reading, feature-kind, and terrain-kind variants the run actually emitted, computed from the output.

#### Scenario: The census is computed

- **WHEN** a run emits paragraphs and headings and nothing else
- **THEN** its final census lists exactly those content variants

#### Scenario: The envelope version is the SDK's

- **WHEN** a plugin's output is read
- **THEN** its `fragment_format` is the version the SDK it was built with emits

## Decisions

- **2026-10-29 — Fragment types live in `transload`, beside the Bundle grammar.**
  The crate owns the transformations in and out of the store; the data model stays IO-free and defines only how a Mark is represented.
  Rejected: Fragment types in the data model (an importer concern in the IO-free crate); a separate crate for the Fragment grammar (moves the grammar away from the crate whose job it is).
  See design D1.
- **2026-10-29 — Mirror only the identity-bearing shapes; 2026-10-30 — name each mirror after what it mirrors, and mirror select options and groups too.**
  Content, rich text, icons, and scalar readings are reused unchanged; the Landmark header, readings, feature kinds, select options and groups, and Terrain lines are mirrored with keys where ids were, as `<Descriptor>Fragment`.
  Rejected: generic id-parameterized data-model types; untyped JSON; `Fragment<Descriptor>` naming; reusing `SelectOption`/`SelectGroup`, which carry a glyph id.
  See design D1.
- **2026-10-30 — Protocharts, not Charts; the Landmark binding is optional.**
  A plugin describes what it found; a person decides what it becomes; a plain page needs no description at all.
  Rejected: plugins emitting Charts and binding by `ChartID` or by key (ties the grammar to builtin id spelling and asks the plugin to know Waypoints' Charts); a mandatory binding with an empty protochart for plain pages.
  See design D2.
- **2026-10-30 — Four key scopes.**
  Rejected: one flat space (collisions between a property and a page named alike); structured keys.
  See design D3.
- **2026-10-30 — No reading-kind Terrain lines in Fragments.**
  A reading edge is a derived index of a reading; a derived index is the core's to build, and nothing in the application builds one today.
  Rejected: carrying them as Bundles do (mirrors a vestige and lets a plugin contradict its own reading); synthesizing them in the SDK.
  See design D5.
- **2026-10-30 — `id` is reserved; other unknown fields are ignored; byte-stability is claimed per run and per SDK-written output, not per value.**
  Rejected: refusing all unknown fields (forbids additive evolution); ignoring `id`; "equal values, equal bytes" (false for ordered maps).
  See design D13.
