# Waypoints: Architecture Reference

> This document describes the data architecture of Waypoints at a level sufficient for an agent to understand what the system can and cannot represent, what constraints exist, and where the boundaries are. It does not cover framework choices, build systems, or deployment — those are implementation concerns. This document covers the **data model**: what structures exist, how they relate, and what their capabilities and limitations are.

---

## 1. Overview

Waypoints is a block-based knowledge management system. Every piece of user-facing data is either a **Mark** (a node in the knowledge graph) or a **Beacon** (a record in the temporal activity layer). Marks relate to each other through **Terrain** (directed edges). Marks that carry structured, typed data conform to **Charts** (schema definitions).

The system is implemented in Rust (authoritative backend) with a Vue/Nuxt frontend rendered via Tauri. SQLite is the persistence layer. The Rust domain model is the source of truth — the frontend is a rendering surface that routes edits back to Rust.

### Crate Structure

The Rust backend is organized into domain-focused crates:

- **`domain`** — Core data structures: Mark, MarkData (Content/Landmark), Chart, Feature, FeatureKind, Terrain, TerrainKind, Reading, RichText, TextMark. No database dependencies. This is the pure domain model.
- **`store`** — SeaORM entity definitions (`entities/`), migrations (`migrations/`), seed data, and query layer (`query/`). Contains the `BuiltinRegistry` for built-in Charts.
- **`cartograph`** — Orchestration layer. Contains the editor subsystem (`editor/` — mutation, history, projection, undo/redo), and handles for charts, marks, and territories. Re-exports editor types (`EditorHandle`, `EditorResult`, `RenderNode`, etc.) as public API. Also owns `WaypointsRuntime` which manages the active territory and database lifecycle.
- **`importer`** — Markdown parsing and import (Waypoints format, GitHub Flavored Markdown with GFM alert → callout mapping).

Type bindings are hand-maintained TypeScript in `crates/domain/typescript/` (`types.g.ts`, `schema.g.ts`). The frontend consumes these via the `@waypoints/domain` package. Tauri commands are defined in `waypoints/src-tauri/src/commands/` and invoked through a typed IPC layer (`waypoints/app/ipc/`).

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

UUIDv4. Generated on creation, never changes. All references to a Mark (from Terrain edges, from Landmark children, from RichReference fields) use the MarkID. This means renames, content changes, and type changes do not break references.

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
    children: Vec<RichReference>,             // ordered child references
    readings: IndexMap<FeatureID, Reading>,   // user-defined field values
}
```

### Guaranteed Fields

Every Landmark, regardless of Chart, has `title`, `icon`, and `children`. These come from the base Page Chart that all Charts conceptually extend. You never need to define these in a Chart — they exist automatically.

### Children

`children` is a `Vec<RichReference>`, where each `RichReference` is:

```
RichReference {
    id:           MarkID,              // the referenced Mark
    display_mode: ReferenceDisplayMode // how to render it
}
```

Display modes: `Avatar`, `Mention`, `Inline`, `Embed`, `CardXS`, `CardSM`, `CardMD`, `CardLG`, `CardXL`

Children represent the ordered content "inside" a Landmark — the blocks that constitute a page's body, a meeting's notes, etc. This is distinct from Terrain edges: children are an ordered list owned by the Landmark, while Terrain edges are graph-level relationships between arbitrary Marks.

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

- **Built-in:** `"builtin.page"`, `"builtin.task"`, etc. Fixed identifiers for Charts that ship with Waypoints. Currently only `Page` is fully implemented; others (Task, Note, Person, Meeting, etc.) are planned.
- **Custom:** 4-character base-36 strings (e.g., `"a1b2"`). Generated for user-defined Charts.

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

### BuiltinRegistry

Built-in Charts are managed through a `BuiltinRegistry` that allows:

- Looking up built-in Charts by ID
- Extending a built-in (creating a custom copy with a new ID)
- Resetting a modified built-in back to its original definition

Eight built-in Charts ship with Waypoints: Page, Objective, Note, Person, Campaign, Meeting, Bookmark, and Journal. Each is defined in `crates/store/src/builtin/`.

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
    src_id: MarkID,         // source Mark
    dst_id: MarkID,         // destination Mark
    kind:   TerrainKind,    // relationship type
    extra:  Json | null,    // edge metadata
}
```

### TerrainKind

Currently three variants:

- **`Child`** — parent-child hierarchy. The source contains the destination. This is distinct from Landmark `children` (which is an ordered list within the Landmark's data). Child Terrain edges represent structural containment in the graph.
- **`Reference`** — a `[[wiki-style]]` link. The source references the destination. This is the primary mechanism for building the knowledge graph's non-hierarchical connections.
- **`Custom`** — application-defined or user-defined relationships. The `extra` field on the Terrain carries any metadata needed to distinguish custom edge types.

### Primary Key

Terrain's primary key in the database is `(src_id, dst_id)`. This means there can be **at most one Terrain edge** between any ordered pair of Marks in the current schema. The multigraph capability (multiple edges between the same pair) described in the domain documentation is a conceptual goal but is **not yet supported by the database schema** — it would require adding `kind` (or a separate edge ID) to the primary key.

### Limitations

**No edge-level ordering.** Terrain edges between a source and its destinations have no intrinsic order. If ordering matters (e.g., the order of child blocks in a document), it must be maintained separately — currently via the `children` field on Landmarks (which is an ordered `Vec<RichReference>`) and via fractional indexing for Content ordering within a document.

**TerrainKind is coarse.** There are only three kinds. Custom edges rely entirely on the `extra` JSON field for semantics. There is no schema validation for custom edge metadata. If the system needs to efficiently query "all edges of custom type X," it must parse the `extra` field, which is not indexed.

**The `(src_id, dst_id)` primary key constraint** means you cannot have both a Child and a Reference edge between the same ordered pair of Marks in the current schema. This is a known limitation that may need to be addressed as the graph becomes richer.

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

1. **No query/view system.** There is no mechanism for "show me all Landmarks of Chart X where Feature Y has value Z." Aggregated contextual views depend on this. The database schema supports it (SeaORM queries), but no query API or view abstraction exists.

2. **Terrain multigraph is conceptual, not implemented.** The `(src_id, dst_id)` primary key prevents multiple edges between the same pair. This needs to be addressed before features like "this Mark is both a child of and a reference from the same parent" can work.

3. **Formula fields have no evaluation engine.** The FeatureKind exists, the Reading storage exists, but no computation happens.

4. **No Beacon table or architecture.** The temporal activity layer described in the feature context document does not yet have a database table, domain type, or any implementation. Health data, activity tracking, and temporal queries are entirely future work.

5. **No full-text search infrastructure.** SQLite FTS5 or similar is not set up. Full-text search across Marks requires this.

6. **Logseq/Notion/Obsidian importers not implemented.** The `importer` crate has Waypoints Markdown and GFM parsing, but no external tool importers exist yet.

7. **The `extra` field on Marks and Terrain is unindexed and unvalidated.** Any feature that relies on querying or filtering by `extra` contents will be slow (full-table scan with JSON parsing) unless auxiliary indexing is added.

8. **WAL mode not explicitly configured.** SQLite WAL checkpoint is called but `PRAGMA journal_mode = WAL` is not explicitly set on connection. Crash recovery testing has not been performed.

9. **No backup/export of territory databases.** No mechanism to export, import, or back up a territory's SQLite database.
