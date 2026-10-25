# Waypoints: Architecture Reference

> This document describes the data architecture of Waypoints at a level sufficient for an agent to understand what the system can and cannot represent, what constraints exist, and where the boundaries are. It does not cover framework choices, build systems, or deployment — those are implementation concerns. This document covers the **data model**: what structures exist, how they relate, and what their capabilities and limitations are.

---

## 1. Overview

Waypoints is a block-based knowledge management system. Every piece of user-facing data is either a **Mark** (a node in the knowledge graph) or a **Beacon** (a record in the temporal activity layer). Marks relate to each other through **Terrain** (directed edges). Marks that carry structured, typed data conform to **Charts** (schema definitions).

The system is implemented in Rust (authoritative backend) with a Vue/Nuxt frontend rendered via Tauri. Persistence is **turso**, accessed through the `bedrock` typed-document layer. The Rust domain model is the source of truth — the frontend is a rendering surface that routes edits back to Rust.

### Crate Structure

The Rust backend is organized into domain-focused crates:

- **`domain`** — Core data structures: Mark, MarkData (Content/Landmark), Chart, Feature, FeatureKind, Terrain, TerrainKind, Reading, RichText, TextMark. Depends on `bedrock` as a façade only, to derive `Collection`/`Scalar` on domain types; it carries no query or connection logic.
- **`store`** — Persistence: `query/` (`mark.rs`, `chart.rs`, `terrain.rs`), builtin Chart definitions (`builtin/`), and seeding (`collections/`). Built on `bedrock`. Exposes an opaque `Store` handle.
- **`cartograph`** — Orchestration layer. Contains the editor subsystem (`editor/` — mutation, history, projection, undo/redo), the app-side sync driver (`sync/`), and handles for charts, marks, and territories. Re-exports editor types (`EditorHandle`, `EditorResult`, `RenderNode`, etc.) as public API. Also owns `WaypointsRuntime` which manages the active territory and database lifecycle.
- **`transload`** — The Bundle format: `export` writes a store as human-legible markdown with YAML frontmatter and JSON sidecars; `ingest` reads one back, all or nothing. The markdown dialect itself lives in `domain::syntax`.
- **`bedrock-core`, `bedrock-derive`, `bedrock`** — Typed document storage over turso. Collections, derive macros, queries (including JSON-path filters), pooling, and durability. `domain` depends only on the `bedrock` façade.
- **`outpost-protocol`, `outpost-fabric`, `outpost-hold`, `outpost`** — The E2EE local-first sync engine. `outpost-protocol` is pure (no tokio/iroh/bedrock). See §11.

Layering note: **Waypoints uses outpost, never the reverse.**

Type bindings are **specta-generated** TypeScript in `crates/domain/typescript/` (`types.g.ts`, `schema.g.ts`). The frontend consumes these via the `@waypoints/domain` package. Tauri commands are defined in `src-tauri/src/commands/` and invoked through a typed IPC layer (`src/app/utils/ipc/`); commands are thin wrappers — test the crates, not the Tauri harness. For browser-based development without Tauri, `cargo xtask dev-server` (in `tooling/xtask`) exposes the same commands over HTTP at `/ipc/<command>` — dev-only, not a product surface.

---

## 2. Marks

A Mark is the atomic unit of the knowledge graph. Every Mark has three fields:

```
Mark {
    id:    MarkID,      // UUIDv4, globally unique, immutable after creation
    data:  MarkData,    // either Content or Landmark
    extra: Json | null, // schema-less metadata bag
}
```

### MarkID

UUIDv4. Generated on creation, never changes. All references to a Mark (from Terrain edges, from RichReference fields) use the MarkID. This means renames, content changes, and type changes do not break references.

### MarkData

A tagged enum with two variants:

```
MarkData = Content(Content) | Landmark(Landmark)
```

Every Mark is exactly one of these. The discriminator serializes as `{ "mark": "content", ... }` or `{ "mark": "landmark", ... }`.

Each variant produces a `type_name()` string used for dispatch:

- Content: `"content.paragraph"`, `"content.heading1"`, `"content.code"`, etc.
- Landmark: `"landmark.{chart_name}"`, e.g., `"landmark.page"`, `"landmark.person"`

### The `extra` Field

Schema-less JSON. Intended for metadata that doesn't fit the Content/Landmark structure — rendering hints, UI state, plugin data. Not validated. Not indexed. Should not be relied upon for core domain logic; it exists as an escape hatch.

---

## 3. Content

Content marks are the rich-text primitives. They are a closed enum — new Content variants require a code change (adding a variant to the Rust enum). This is by design: Content represents the fixed vocabulary of block types that Waypoints natively understands.

### Current Variants

Every Content variant carries an optional `theme: Option<BlockTheme>` for block-level color theming (foreground, background, and border derived from a single `Color` value via `data-theme` CSS attributes). Omitted from the table for brevity.

| Variant      | Fields                                                   | Notes                                              |
| ------------ | -------------------------------------------------------- | -------------------------------------------------- |
| `Paragraph`  | `rich_text: RichText`                                    | Default block type. Most common.                   |
| `Heading1`   | `rich_text: RichText`                                    | Separate variants per level (not a `level` field). |
| `Heading2`   | `rich_text: RichText`                                    |                                                    |
| `Heading3`   | `rich_text: RichText`                                    |                                                    |
| `Blockquote` | _(none)_                                                 | Compound block — children via Terrain, not inline. |
| `ListItem`   | `rich_text: RichText`                                    | Unordered list item.                               |
| `EnumItem`   | `rich_text: RichText`                                    | Ordered list item.                                 |
| `Callout`    | `icon: String`                                           | Compound block — children via Terrain, not inline. |
| `Code`       | `language: String`, `lines: Vec<String>`                 | No RichText — plain strings per line.              |
| `Table`      | `cells: Vec<Vec<RichText>>`, sizing/header/color options | 2D grid of RichText cells. Stub UI only.           |
| `Math`       | `expression: String`, `engine: MathEngine`               | Engine is `Typst` (default) or `LaTeX`.            |
| `Divider`    | _(none)_                                                 | Horizontal rule. No data.                          |

### Compound Blocks

Blockquote and Callout are **compound blocks** — they contain child blocks rendered inside them. Children are stored as Terrain `Child` edges (same as document-level nesting), not as inline `Vec<Content>`. The frontend collects descendants from the flat block list by parent ID and renders them recursively. Compound blocks provide `compound-parent-depth` context so children compute their visual indentation relative to the compound container.

### Limitations and Constraints

**Content is a closed set.** You cannot define new Content variants without modifying the Rust enum. This is intentional — Content represents Markdown-equivalent primitives that have known rendering behavior. If you need a new "type" of block with custom fields, that's a Landmark with a Chart, not a new Content variant.

**Headings are separate variants, not parameterized.** `Heading1`, `Heading2`, `Heading3` are distinct enum arms. This simplifies pattern matching and TipTap extension mapping (each heading level is its own node type) but means adding `Heading4`+ requires a code change.

**Code blocks have no RichText.** They store plain `Vec<String>` lines. No inline formatting inside code blocks. Syntax highlighting is a frontend concern (Shiki) driven by the `language` field.

**Tables store data, not layout.** The `cells` grid holds content; sizing, headers, and color are optional metadata. Tables are not spreadsheets — no formulas, no cell references. UI is currently a stub.

---

## 4. RichText and TextMarks

Most Content variants that hold text use the `RichText` type:

```
RichText {
    plain_text: String,       // the raw text content
    marks: Vec<TextMark>,     // formatting ranges overlaid on the text
}
```

### TextMark

```
TextMark {
    kind:  TextMarkKind,   // what kind of formatting
    start: u32,            // Unicode character offset, inclusive
    end:   u32,            // Unicode character offset, exclusive
    data:  Json | null,    // additional data (e.g., URL for links)
}
```

TextMarks are **offset-based ranges** that overlay formatting on the plain text. They are allowed to overlap — you can have bold from 0..10 and italic from 5..15, producing a bold-italic region from 5..10. This is fundamentally different from tree-based markup (HTML/XML) where nesting must be well-formed.

### TextMarkKind

The current set of inline formatting types:

`Bold`, `Italic`, `Underline`, `Strikethrough`, `Superscript`, `Subscript`, `Code`, `Link`, `Mention`, `Highlight`, `Color`, `FontSize`, `FontFamily`, `Math` (inline math), `Spoiler`, `Comment`

Some kinds carry data in the `data` field:

- `Link`: `{ "url": "..." }`
- `Mention`: reference metadata
- `Highlight` / `Color`: color value
- `FontSize` / `FontFamily`: specific values
- `Comment`: comment content/metadata

### Limitations

**TextMarks are not semantic.** They are presentation-layer formatting. A `Mention` TextMark is not the same as a Terrain Reference edge — the TextMark controls how text renders, while a Terrain edge is a structural relationship in the graph. Both may exist for the same conceptual reference, but they serve different purposes.

**Offsets are Unicode character positions**, not byte positions. This matters for Rust/JS interop since Rust strings are UTF-8 bytes and JavaScript strings are UTF-16.

---

## 5. Landmarks

Landmark marks are instances of Charts (schema definitions). Where Content is a closed set of primitives, Landmarks are an open set — any Chart defines a new kind of Landmark.

```
Landmark {
    chart:    ChartID,                        // which Chart this conforms to
    title:    RichText,                       // always present
    icon:     Icon | null,                    // typed icon
    readings: IndexMap<FeatureID, Reading>,   // user-defined field values
}
```

### Guaranteed Fields

Every Landmark, regardless of Chart, has `title` and `icon`. These come from the base Page Chart that all Charts conceptually extend. You never need to define these in a Chart — they exist automatically.

### Content

**Terrain `Child` edges are the only expression of containment and ordering.**
A Landmark's content — the blocks constituting a page's body, a meeting's
notes, etc. — is carried entirely by Terrain `Child` edges, ordered by their
`order` fractional index. `projection::project_landmark` walks
`terrain::get_child_edges`; no field of the Landmark contributes.

> **Historical note:** Landmark data used to carry a
> `children: Vec<RichReference>` field — a leftover from an earlier design
> that was never written or read by production code. It was **removed** so
> containment is structurally single-authority (a field one device populates
> and another ignores is a silent-divergence hazard under sync). Rows
> persisted before the removal may still carry a `"children"` key in stored
> JSON; it is ignored on read and disappears the next time the row is
> written. Do not re-add such a field.

`RichReference` itself survives (`id: MarkID` + `display_mode:
ReferenceDisplayMode`, modes: `Avatar`, `Mention`, `Inline`, `Embed`,
`CardXS`–`CardXL`). It is the typed payload intended for inline Mark
references inside `Content` marks, which today travel untyped on
`InlineMark.data`; `ReferenceDisplayMode` is live via `FeatureKind::Reference`.

### Readings

`readings` is an `IndexMap<FeatureID, Reading>` — an ordered map of field values, keyed by Feature ID (not feature name). This means renaming a Feature does not break stored data.

---

## 6. Charts

Charts are schema definitions for Landmarks. They define what fields (Features) a Landmark of that type has.

```
Chart {
    id:          ChartID,                       // "builtin.*" or 4-char base-36
    name:        String,                        // human-readable
    description: String | null,
    icon:        Icon | null,
    features:    IndexMap<FeatureID, Feature>,   // ordered field definitions
}
```

### ChartID

Two kinds:

`ChartID` is a plain `String` newtype (`ChartID(pub String)`) — the format is a convention, not a type-level guarantee.

- **Built-in:** short generated string codes, **not** dotted `"builtin.*"` paths. Observed in the seeded data: `nage` (Page), `nerson` (Person), `nook` (Book). All nine built-ins are implemented and seeded, not planned.
- **Custom:** short generated string codes, same space. Generated for user-defined Charts.

> The `"builtin.*"` dotted form still appears in `type_name()` strings (`"landmark.page"`) and in mark-kind filtering, which is a _different_ string from the ChartID. Don't conflate them — see `crates/store/src/query/mark.rs`.

### Features

```
Feature {
    id:          FeatureID,       // 4-char base-36, stable identifier
    name:        String,          // human-readable label
    kind:        FeatureKind,     // determines validation and UI
    description: String | null,
    required:    bool,
}
```

### FeatureKind

The type system for Landmark fields. This is a closed enum — adding new kinds requires a code change.

| Kind                 | Description                                                               | Reading Type                         |
| -------------------- | ------------------------------------------------------------------------- | ------------------------------------ |
| `Text`               | Plain text                                                                | `String`                             |
| `Number`             | Numeric with optional min/max/step/precision                              | `f64`                                |
| `Checkbox`           | Boolean                                                                   | `bool`                               |
| `Date`               | Date/datetime with optional range constraints and timezone                | `NaiveDateTime` or range             |
| `Email`              | Email with validation                                                     | `String`                             |
| `Phone`              | Phone number                                                              | `(String, String)` (number, country) |
| `URL`                | URL with validation                                                       | `String`                             |
| `Select`             | Single choice from options                                                | `GlyphID`                            |
| `MultiSelect`        | Multiple choices with optional max                                        | `Vec<GlyphID>`                       |
| `GroupedSelect`      | Single choice from grouped options (state machine style)                  | `GlyphID`                            |
| `GroupedMultiSelect` | Multiple choices from grouped options                                     | `IndexMap<GlyphID, GlyphID>`         |
| `Reference`          | Link(s) to other Landmarks, with optional type filtering and display mode | `Vec<MarkID>`                        |
| `Formula`            | Computed field (not yet implemented)                                      | `String` (expression)                |

### Select Options and Glyphs

Select-type fields use `GlyphID` (4-char base-36) to identify options. Each option has:

```
SelectOption { id: GlyphID, name: String, description: String | null, icon: Icon | null, color: Color }
```

Grouped selects organize options into `SelectGroup`s, each with their own max_selections and default_options. This supports state-machine-like patterns (e.g., a Task status field with groups like "Not Started", "In Progress", "Complete", each containing sub-states).

### Reference Features

The `Reference` FeatureKind has several configuration options:

- `allowed_types: Vec<ChartID>` — restricts which Chart types can be referenced
- `max_references: u32` — caps how many references can be stored
- `display_field: String` — which field to show when rendering the reference (default: `"title"`)
- `display_mode: ReferenceDisplayMode` — default rendering style
- `filter_criteria: Json` — optional filtering constraints

### Built-in Charts

**There is no `BuiltinRegistry`** — it was removed. Built-in Charts are plain Chart values defined one-per-module in `crates/store/src/builtin/` and seeded via idempotent `upsert` on every boot (`crates/store/src/collections/seed.rs`). "Extend a builtin" and "reset a builtin" are not implemented.

**Nine** built-in Charts ship: Page, Objective, Note, Person, Campaign, Meeting, Bookmark, Journal, and **Book**.

> Note: this set has never been reconciled against the dogfooding set named in the planning material (Page, Task, Daily Note, Meeting, Person, Research Paper, Project). Treat the nine above as what the code actually seeds, and the mismatch as an open question rather than a settled decision.

### Limitations

**Charts are flat, not hierarchical.** There is no Chart inheritance beyond the implicit Page base. A Task Chart cannot "extend" a Meeting Chart. If you want shared fields, you duplicate them across Charts (or use the base Page fields that all Charts already share).

**FeatureKind is a closed set.** If you need a field type that doesn't exist (e.g., a geographic coordinate, a duration, a currency amount), you must either use an existing kind approximately (Number for coordinates, Text for formatted values) or add a new FeatureKind variant to the Rust enum.

**Formula fields are defined but not implemented.** The `FeatureKind::Formula` variant exists in the enum, but no evaluation engine exists yet.

**Validation is per-field, not cross-field.** Each Reading is validated against its FeatureKind independently. There is no mechanism for "this field is required only if that other field has value X" or other cross-field constraints.

---

## 7. Terrain

Terrain represents directed edges between Marks. The combination of Marks (nodes) and Terrain (edges) forms a multigraph — multiple edges with different kinds can exist between the same pair of Marks.

```
Terrain {
    id:     TerrainID,      // stable edge identity — the primary key
    src_id: MarkID,         // source Mark        (indexed)
    dst_id: MarkID,         // destination Mark   (indexed)
    kind:   TerrainKind,    // relationship type  (indexed, stored as a flattened string)
    order:  String | null,  // fractional index; meaningful for ordered kinds like Child
    extra:  Json | null,    // edge metadata
}
```

### TerrainKind

Four variants:

- **`Child`** — parent-child hierarchy. The source contains the destination. Child Terrain edges are the sole expression of structural containment in the graph; Landmark data carries no child list.
- **`Reference`** — a `[[wiki-style]]` link. The source references the destination. The primary mechanism for the graph's non-hierarchical connections.
- **`Custom`** — application- or user-defined relationships. The `extra` field carries any distinguishing metadata.
- **`Reading(ChartID, FeatureID)`** — a `Reference`-kind Reading materialized as a graph edge. The `ChartID`/`FeatureID` identify _which_ Reference feature on _which_ Chart the edge represents, enabling attributed queries ("all backlinks into this landmark via the `attendees` feature of a `Meeting`") without scanning every landmark's readings JSON.

### Primary Key — the multigraph IS implemented

Terrain's primary key is **`id: TerrainID`**, a stable per-edge identity generated on creation and never changed. `src_id`, `dst_id`, and `kind` are each indexed but none is part of the key.

This means multiple edges of different kinds — or even the same kind — may exist between the same ordered pair of Marks. The multigraph is a shipped capability, not a conceptual goal. Address a specific edge by its `TerrainID`.

### Limitations

**Edge ordering exists but is kind-specific.** Terrain carries an `order: Option<String>` fractional index, used for ordered kinds (notably `Child`). It is not meaningful for every kind, and edges of unordered kinds have no intrinsic order.

**Terrain `Child` edges are the sole authority for containment.** The former `Landmark.children` field that looked like a competing source of truth has been removed — see §5. Child ordering lives in the edge's `order` fractional index, not on the Landmark.

**Custom edges have no schema validation.** `Custom` relies entirely on the unindexed `extra` JSON field for semantics, so "all edges of custom type X" requires a scan with JSON parsing. Note that `Reading(ChartID, FeatureID)` exists precisely to avoid this for the reference-reading case.

---

## 8. Identifiers

Waypoints uses several ID types, all designed around stability (renaming things doesn't break references):

| ID Type     | Format                          | Scope       | Notes                                                                                          |
| ----------- | ------------------------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| `MarkID`    | UUIDv4                          | Global      | Every Mark gets a unique, immutable UUID.                                                      |
| `ChartID`   | `"builtin.*"` or 4-char base-36 | Global      | Built-ins use string prefixes; custom uses generated base-36.                                  |
| `FeatureID` | 4-char base-36                  | Per-Chart   | Unique within a Chart. Keying Readings by FeatureID (not name) means renames don't break data. |
| `GlyphID`   | 4-char base-36                  | Per-Feature | Identifies select options. Same stability guarantee.                                           |

The 4-character base-36 format gives ~1.68 million possible values per ID space, which is sufficient for per-Chart and per-Feature scoping.

---

## 9. Color and Icon Systems

### Color

A closed enum of 10 named colors: `Default`, `Gray`, `Brown`, `Orange`, `Yellow`, `Green`, `Blue`, `Purple`, `Pink`, `Red`. Used for callout backgrounds, select option colors, table row/column colors. Not arbitrary hex — intentionally constrained for visual consistency.

### Icon

```
Icon {
    identifier: String,     // hierarchical (e.g., "iconify:mdi:home")
    alt_text:   String?,    // accessibility
    colorable:  bool?,      // can the UI tint this icon?
    metadata:   Json?,      // additional rendering hints
}
```

Icon namespaces: `Iconify` (icon libraries), `Waypoints` (custom, future), `SystemEmoji`, `LocalImage`, `RemoteImage`.

---

## 10. What the Architecture Cannot (Yet) Do

This section enumerates known gaps and missing capabilities. An agent should reference this when evaluating feature proposals.

1. **No user-facing query/view system.** A programmatic query layer _does_ exist — `crates/store/src/query/` over bedrock, with equality filters, sorting, and JSON-path filters (`Filter::json_eq`, lowering to `json_extract`). What does not exist is any abstraction for "show me all Landmarks of Chart X where Feature Y has value Z" as a user-defined, saved, or composable view. Aggregated contextual views depend on that layer, not on raw query capability.

2. ~~**Terrain multigraph is conceptual.**~~ **Resolved — the multigraph is implemented.** Terrain's PK is `id: TerrainID`; see §7.

3. **Formula fields have no evaluation engine.** The FeatureKind exists, the Reading storage exists, but no computation happens.

4. **No Beacon table or architecture.** The temporal activity layer described in the feature context document does not yet have a database table, domain type, or any implementation. Health data, activity tracking, and temporal queries are entirely future work.

5. **No full-text search infrastructure.** SQLite FTS5 or similar is not set up. Full-text search across Marks requires this.

6. **Logseq/Notion/Obsidian importers not implemented.** `transload` ingests Waypoints' own Bundle format; no importer for an external tool exists yet.

7. **The `extra` field on Marks and Terrain is unindexed and unvalidated.** Any feature that relies on querying or filtering by `extra` contents will be slow (full-table scan with JSON parsing) unless auxiliary indexing is added.

8. **No backup/export of territory databases.** No mechanism to export, import, or back up a territory's database. The engine's durability posture itself is specified and verified — WAL, `synchronous = FULL`, and committed writes surviving a SIGKILLed process (`openspec/specs/storage/spec.md`, _Committed writes survive abrupt process termination_; `crates/bedrock/tests/crash_recovery.rs`, `crates/bedrock/tests/durability.rs`) — so what is missing is tooling, not confidence in the files.

9. **Note content does not sync.** The sync engine works, but nothing serializes the Mark/Terrain graph onto it. See §11.

---

## 11. Sync (outpost)

Sync is provided by **outpost**, an end-to-end-encrypted local-first sync engine living in four crates. It is a separate architecture from the Mark model and is deliberately **payload-agnostic** — it moves opaque encrypted blobs and knows nothing about Marks.

### Roles and vocabulary

| Term          | Meaning                                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Territory** | The replicated unit — one user's synced dataset. Identified by `TerritoryID`.                                       |
| **Vessel**    | A device that can author. Holds the identity, can encrypt/decrypt, originates changes.                              |
| **Station**   | An always-online peer that stores and serves encrypted Notices. Validating but **content-blind**; never originates. |
| **Fleet**     | A named, key-identified set of Stations. The deployment/trust boundary.                                             |
| **Notice**    | One signed, content-addressed, encrypted operation record. Identified by `NoticeID` (a content hash).               |
| **Track**     | A connected component of the Notice DAG, rooted at a parentless Notice. `TrackID` **is** that root's `NoticeID`.    |

Identity derives from a BIP-39 mnemonic (`KeyRoot` → `Identity` + per-device `DeviceKey`). Membership is governed by an ACL log; read keys are versioned by **epoch** and rotate when a member is removed.

### Ordering — this is the part that trips people up

- **Notices have no total order.** They form a DAG ordered only by parentage. The per-Fleet _orderer_ totally-orders **ACL records only**, never Notices.
- A Notice's Track is assigned **structurally** on ingest: no parents ⇒ it starts a new Track; otherwise it inherits `parents[0]`'s Track. The app's only lever is what it passes as `parents` when authoring.
- `EnvelopeHeader.timestamp` exists, is signed and AAD-bound, and is documented "advisory only, never ordering." It is currently unused.
- Consequence: **any merge law must derive its own ordering** — DAG parentage establishes causality, not which of two concurrent edits wins.
- Tracks are a **sync-partitioning** mechanism (divergence detection keys on `TrackID`), not a semantic boundary.

### Two Track mistakes worth naming

**A Notice cannot be in two Tracks.** Ingest assigns exactly one, inherited from `parents[0]`. Naming two Tracks' heads as parents does not place the Notice in both — `append_notice` clears `is_head` on _every_ parent, so the second Track is left headless and fuses into the first. Related consequence: because an operation spanning two Landmarks must land in one Track or the other, Tracks gradually **fuse** as blocks move between pages. That erodes partitioning quality over a Territory's lifetime. It is never a correctness problem (the fold is Territory-wide and keys on operation target) and it is not fixable by re-partitioning, since content addressing forbids rewriting parentage.

**A Track is not "the history of a page."** It closely resembles one, which is why this recurs. It isn't, for two reasons: a cross-Landmark operation appears in only one of the two Tracks it concerns, and Tracks fuse over time. Derive any history view — page history, per-field change log, "what changed while I was offline" — by **filtering folded operations on their target**, never by reading `track_notices`. A `track_notices` view is _incomplete_ rather than obviously broken, so it ships looking correct and degrades quietly. Treat `track_notices` as transport debugging only.

### Reachability posture

Endpoints currently bind **loopback only**; the routable bind path exists but is unused. This was a deliberate development-ergonomics choice, not an oversight. Practical effect: **no two physical devices have ever synced** — all verification to date is same-host.

Stations are **accept-only** — they never dial a Vessel, though they push freely over a session a Vessel opened. So Vessels need no inbound reachability, and NAT traversal is not on the critical path for device↔Station sync.

### The gap

The `Maneuver` op enum (`crates/cartograph/src/editor/maneuver.rs`) describes every graph mutation and is currently used **only for local undo/redo**. Nothing serializes it into a Notice; `EditorService` has no sync coupling. The app-side driver (`crates/cartograph/src/sync/`) handles identity, pairing, Station config, and the device registry — the plumbing around sync, not the syncing of content.

**Therefore: identity and pairing sync; notes do not.** The merge law, the payload wiring, and the reachability work are all in active design. Do not assume any of it when evaluating a proposal — check the current change set first.

### Explicitly not provided

No character-level text CRDT, no real-time collaborative editing, no snapshot/compaction layer (log growth is unbounded by design for now), and no Vessel-to-Vessel sync (a stated architectural goal, but every current flow routes through a Station).
