# 📐 projection-spec — Draft 0.3 (2026-08-18 local carriage)

> **Status: historical drafting record (demoted 2026-08-19).** This document
> deliberately mixed concerns — importer expectations and the markdown
> export — as a brainstorm carrier; its durable content has since been split
> into targeted, standalone forms: the markdown-dialect reference and the
> export-format contract, with the testable requirements restated in this
> change's `specs/` deltas (which are normative for the change — where this
> document disagrees with them, the deltas win). It remains the sole carrier
> of the importer-side material (§9 ledger and identity-resolution
> semantics) until `importer-abi` is pulled down.
>
> **Provenance.** Pulled from the Notion `projection-spec` page
> (https://app.notion.com/p/3be69fa1cfc881deaedce8b140c32d64) on 2026-08-18,
> with the 0.2→0.3 redline resolved to its current text (struck text dropped,
> additions kept). The Notion page retains the redline history. Normative
> language per RFC 2119 (MUST/SHOULD/MAY).
>
> **Status:** Draft 0.3 · 2026-08-17. Serves as the technical annex to the
> data covenant, the source document for `/docs/export`, and the importer
> contract (a plugin is anything that produces a valid Bundle or Fragment).
> Decision markers: ◆ = open for Draft 0.4, collected in §12.
>
> **Local carriage notes** (divergences known at pull time, not edits to the
> spec text):
>
> - §9.2 specifies the ledger as a materialized view over `extra` via
>   bedrock's `experimental_materialized_views`. `FINDINGS.md` V1 measured
>   that the pinned turso engine does not maintain MVs post-declaration;
>   design.md D4 records the Phase-2-binding replacement (ordinary core-owned
>   table, rebuildable from the same namespaced `extra`). The spec text below
>   is carried as written.
> - §3 places a Chart's `extra` inside `_schema.json`. design.md D3
>   (proposed — ratify at review) locates it under a top-level `x-waypoints`
>   member alongside the canonical Chart, rather than as a bare top-level key.
> - One code comment in §3's layout block arrived mangled by the redline
>   (`library/ # user to avoid potential collisions with ChartIDs`) and was
>   normalized to its evident intent.
> - In-session rulings after the pull (2026-08-18) supersede two details of
>   the text below, recorded in design.md D7/D8: callouts are Comark
>   **block components**, not admonitions (§4.2's "callouts (Comark
>   admonitions)"), and inline mentions are `[[…]]` **wiki links**.

## 1. The three guarantees

Every rule in this spec serves one of three guarantees, and each rule is
tagged accordingly:

- **[L] Legibility** — the markdown files are a _human-legible projection_.
  Opened in any CommonMark tool, in 1980 or 2180, they read as clean
  documents. Waypoints-specific machinery degrades invisibly.
- **[F] Fidelity** — the Bundle as a whole (files + schemas + sidecars +
  manifest) is a _lossless carrier_. `ingest(export(store)) == store`.
  Nothing about the domain model is unrepresentable. [F] quantifies over
  Waypoints stores exclusively — `store` is a Waypoints store. Logseq,
  Notion, AnyType, and every other foreign platform sit outside [F]
  entirely.
- **[I] Idempotency** — re-running the same import never duplicates. For
  Waypoints Bundles, [I] is a corollary of [F] plus declared ids: same `id`
  ⇒ same Mark, nothing to prove. For importer Fragments, [I] is §9's entire
  job.

The projection is allowed to be lossy; the Bundle is not. When the two
guarantees conflict, the resolution is always the same: the legible thing
stays legible, and the residue moves to a sidecar.

## 2. Terminology

- **Mark** — atomic content unit.
- **Landmark** — typed semantic node; an instance of a **Chart**
  (user-definable schema) whose field definitions are **Features**,
  instantiated as **Readings**.
- **Terrain** — the directed multigraph of edges between Marks.
- **Bundle** — the complete export directory.
- **Projection** — the markdown-file subset of the Bundle.
- **Ingester** — the single trusted Rust path that reads a Bundle into the
  store.
- **Ledger** — the core-owned memory mapping importer source keys to minted
  ids — realized as a materialized view over `extra` (§9.2). Never exported.
- **Fragment** — an importer plugin's output, written in Bundle grammar but
  possibly partial: a single page, one Chart, no manifest. Fragments carry
  `src` and never `id`; export-Bundles carry `id` and never `src` —
  direction-tagging that makes identity forgery a parse error rather than a
  policy (§9).

**Beacons** are out of scope for Bundle format 1.0; append-only series will
get their own carrier (JSONL streams) in a future minor version. This spec
reserves the `beacons/` directory name.

**ID inventory (per `crates/domain`):**

| Identifier  | Shape                               | Scope                                                                   |
| ----------- | ----------------------------------- | ----------------------------------------------------------------------- |
| `MarkID`    | UUID                                | Global — the identity-comment carrier (§4.3) and Landmark filename (§3) |
| `TerrainID` | UUIDv4                              | Global — one per edge; what makes the multigraph a multigraph           |
| `ChartID`   | 4-char base-36, or `builtin.{name}` | Global                                                                  |
| `FeatureID` | 4-char base-36 (`Base36ID`)         | Within its Chart — the file's single `chart:` binding disambiguates     |
| `GlyphID`   | 4-char base-36 (`Base36ID`)         | Within its Feature — select options                                     |

No database-local identifier (rowid, sequence, or similar) ever appears in a
Bundle — every ID above is a domain-level stable handle, minted at creation
and serialized transparently.

## 3. Bundle layout

```bash
<export-root>/
  manifest.json      # §8 — format version lives here
  library/           # named to avoid collisions with ChartIDs
    <ChartID>/       # one directory per Chart
      _schema.json   # the Chart's JSON Schema — carries its extra [F]
      <MarkID>.md    # one file per Landmark owned by this Chart  [L+F]
  assets/
    <hash>.<ext>     # content-addressed                          [F]
    <hash>.<ext>.md  # companion annotation files                 [L+F]
  terrain.jsonl      # edge residue: Tier-3 records + edge extra, §6 [F]
  marks.jsonl        # Mark extra — Landmark roots + Content Marks   [F]
```

**Filename policy.** Keyed by UUID. The exporter MAY additionally emit an
advisory root `index.md` (title → file map), regenerated on every export and
never trusted on ingest [L]. Since the Bundle is not a machine-migration
path, we do not need to export the Ledger (§9). The machine-migration path
is sync, not Bundles.

Landmarks are grouped by their Chart within a "Library". Thus, every
Landmark maps to: `library/<ChartID>/<MarkID>.md`.

**Sidecars follow one rule: each entity's residue — whatever its primary
carrier can't hold — lands in a file named for the entity.** `extra` is
domain data on Charts, Marks, and Terrains, so [F] must carry it — but
inline in the markdown it would turn the projection into an ungodly mess by
the second plugin. So: **Marks** (Landmark roots and Content Marks alike) →
`marks.jsonl`, one line per extra-bearing Mark:

```json
{ "id": <MarkID>, "extra": {…} }
```

**Charts** need no sidecar — `_schema.json` is already a structured
per-entity file, so a Chart's `extra` rides inside it (the mess argument
applies to markdown, not JSON). Draft-07 note, closing the last 0.3
question: unknown keywords in a schema document are spec-legal — validators
SHOULD ignore them, instance validation consults only known keywords, and
the Draft-07 meta-schema itself carries no `additionalProperties: false`, so
a `_schema.json` with a top-level `extra` still validates as a Draft-07
document and still validates frontmatter unchanged. CI check: every exported
`_schema.json` MUST validate against the Draft-07 meta-schema. Tooling
caveat: ajv v8's strict mode lints unknown keywords — declare `extra` or set
`strict: "log"`. **Edges** → `terrain.jsonl` (§6): a separate edge-extra
file would nullify it, since `TerrainKind::Custom` stores its custom data
within `extra` and needs it alongside anyways. Namespaced keys
(`so.waypoints.importer.logseq`, etc.) keep plugins out of each other's
pockets, and provenance survives Bundle migration: re-ingesting a Bundle
rebuilds the ledger's materialized view from these files automatically.

## 4. The Landmark file

Three layers, in order: frontmatter, identity comment, body.

### 4.1 Frontmatter [L + F]

YAML frontmatter carries the Landmark's flattened root Mark (including the
created/updated timestamps, ChartID, scalar Readings, etc.):

```yaml
---
title: Quarterly Review
chart: builtin.meeting # ChartID — base-36 or builtin.*
created: 2026-05-11T14:03:00Z # ISO-8601 wall time            [L]
modified: 2026-08-02T09:41:00Z
readings:
  a1b2: # feature: attendees (advisory)
    - ref: 018f4b77-… # MarkID — Tier-2 edge lives here §6
      label: Jane Doe # advisory, regenerated          [L]
  c4d8: "Room 4" # feature: location (advisory)
  e7f2: |- # feature: notes — rich text
    Discussed **Q3 numbers** and agreed on next steps.
---
```

- Reading keys are `FeatureID`s [F] — 4-character base-36, unique within
  their `Chart`; the file's single `chart:` binding disambiguates —
  rename-invariant by construction. Human names ride as YAML comments:
  advisory, regenerated on every export, never trusted on ingest. The
  `Chart` schema is the normative id → name join. The `Chart` binding is a
  `ChartID`: 4-character base-36, or `builtin.{name}`.
- Scalar Readings serialize as plain YAML values, typed per the Chart's JSON
  Schema.
- Relation Readings serialize as lists of `{ref, label?}`; `ref` is
  normative [F], `label` is advisory [L] and MUST be regenerated on export,
  never trusted on ingest.
- Wall-clock ISO stays in frontmatter as part of the flattened `Mark`.
  Bundles are a freeze-frame carrying knowledge, not sync machinery.
- `RichText` Readings are block scalars: serialize as YAML `|-` blocks
  containing markdown. The exporter normalizes chomping and whitespace so
  round-trips are byte-stable, and segments ↔ markdown runs through the
  single shared parser path in both directions. The body stays purely the
  Tier-1 tree.

### 4.2 Body [L + F]

The body is the Landmark's structural content: the Tier-1 (child-edge) tree
of content Marks, projected to markdown. Block types map to their natural
CommonMark/Comark forms — paragraphs, headings, lists, blockquotes, callouts
(Comark admonitions), math blocks. Content Marks never get their own files;
the document _is_ their serialization.

### 4.3 Identity comments [F]

Every Mark in the body is preceded by an identity comment, Gutenberg-style:

```markdown
<!-- wp:mark {"id":"018f3c2a-…"} -->

The paragraph text of the Mark.
```

- Because export-Bundles are freeze-frames, they must carry `id` and could
  include the remaining, flattened, serialization of `Mark` (similar to
  frontmatter for Landmarks). Unknown fields MUST be preserved by
  round-tripping tools.
- **Shared Marks** appear in every Landmark file that contains them,
  duplicated exactly — identical identity comment, identical content. On
  ingest, same `id` ⇒ same Mark. For divergent duplicates, two rules:
  1. Single-render invariant (normative): the exporter renders each shared
     Mark once and byte-copies it into every containing file — divergence
     within one export is impossible, and testably so.
  2. The one reachable divergence is Bundle surgery (hand-merged files from
     different snapshots): with no HLCs to arbitrate, the ingester MUST
     refuse loudly with a per-UUID conflict report — the covenant's
     defined-refusal clause, one layer down.

  A `--prefer` flag MAY come later.

- **Pairing.** Leaf Marks take a single preceding comment. Container Marks
  (children of mixed types, or nesting that markdown structure alone can't
  disambiguate) take open/close pairs: `<!-- wp:mark … -->` …
  `<!-- /wp:mark -->`. **Always** pair containers, **never** pair leaves.

### 4.4 Attributes vs. comments — the carrier rule [L]

Comark attributes carry content semantics; comments carry identity and
provenance. If a conforming Comark renderer _should render differently_
because of the datum (admonition parameters, inline parameterization,
Reading bindings per §4.1), it goes in attributes. If no renderer should
ever display it (UUIDs, HLCs, importer provenance), it goes in comments.
Consequence: stripping every comment from the projection yields a document
with zero Waypoints residue — pure Comark — and stripping attributes too
yields clean CommonMark. That gradient _is_ the 1980-or-2180 claim, made
mechanical.

## 5. Charts [F]

Each Chart exports as one JSON Schema at `library/<ChartID>/_schema.json`
(§3), carrying Feature definitions, types, and constraints. The schema is
the contract importers write against and the ingester validates against.
Chart schemas are identified by `ChartID` everywhere — the frontmatter
binding and the schema's `$id` match: 4-character base-36, or
`builtin.{name}` for built-ins — and human names are advisory YAML comments.
Renaming a Chart touches nothing normative.

## 6. Edge tiers [F]

Terrain edges export by exactly one of three rules:

| Tier | Edge kind                  | Carrier                           | Guarantee |
| ---- | -------------------------- | --------------------------------- | --------- |
| 1    | Structural (child)         | Document nesting in the body      | [L + F]   |
| 2    | Relational (Reading-bound) | `readings.*[].ref` in frontmatter | [L + F]   |
| 3    | Everything else            | `terrain.jsonl`                   | [F]       |

`terrain.jsonl` carries every edge fact the projection cannot — one line
shape, two roles, dispatched on `kind` by the same non-wildcard match as the
exhaustiveness invariant:

```json
{
  "src": "…",
  "dst": "…",
  "kind": "…",
  "order": …?,   # ordinal among parallel (src, dst, kind) edges
  "extra": {…}?  # namespaced, §3
}
```

**Constitutive** lines (Tier-3 kinds) _are_ the edge record — and since
`Custom` keeps its payload in `extra`, they carry it inline by construction.
**Annotative** lines (Tier-1/2 kinds) exist only for extra-bearing edges
whose structure already lives in the projection: the natural key
`(src, dst, kind, order)` addresses the projection-reconstructed edge, and
the ingester attaches `extra` to it. An annotative line matching no
projected edge is a malformed Bundle — loud refusal, per §4.3's precedent.
`order` is an _ordinal_, not the runtime fractional key: order strings are
sequence machinery that regenerate on ingest, like clocks — the sequence is
the knowledge, the strings are implementation.

**`TerrainID`s regenerate unconditionally.** This is because `TerrainID`s
are only ever necessary at runtime to target specific edges so that changes
don't need to be delete-then-insert, but can instead be a proper update.
Once we export structural and relational edges, the only edges that remain
are either `TerrainKind::Custom` or structural/relational edges with
non-null `extra` fields. The `TerrainKind::Custom` are all unordered, which
means that nothing should be directly referencing them in a structural
capacity. Similarly, the projection of Landmarks into markdown already
encodes the necessary ordering to correctly reconstruct the Terrain edges
with their appropriate `extra` fields, independently of the `TerrainID`.
Additionally, since Bundles are necessarily frozen, there's no need to care
about uniquely identifying them to discern mutations.

**Exhaustiveness invariant (normative):** the exporter's dispatch over edge
kinds MUST be a non-wildcard match. Introducing a new edge kind without an
explicit tier assignment MUST fail compilation. No edge is ever silently
dropped; Tier 3 exists precisely so "we don't know how to make this legible
yet" never becomes "we lost it."

Round-trip rule: the ingester MUST reconstruct Terrain as exactly
Tier 1 ∪ Tier 2 ∪ Tier 3.

Crate validation: Tier-2 is exactly `TerrainKind::Reading(ChartID,
FeatureID)` — persisted as `reading::{chart}::{feature}` — so the
frontmatter position (the file's `chart:` binding plus the `FeatureID` key)
reconstructs the edge kind losslessly. The spec and the domain agree by
construction.

## 7. Assets & annotations

Assets are content-addressed: filename = `<hash>.<ext>`, referenced from
bodies by directory-local links (`../assets/<hash>.png`) so the Bundle
browses by hand [L]. **Hash:** use whatever content-addressing the outpost
already implements. Annotations export as companion markdown files beside
their asset (`<hash>.pdf.md`, per §3), themselves valid Landmark-file
grammar (§4), because annotations are Marks.

## 8. Manifest & versioning [F]

`manifest.json` carries: `bundle_format` (semver — this is the number the
covenant freezes), exporter app version, export timestamp, Territory
identity, entity counts, and OPTIONAL per-file checksums.

Breaking (major): removing/renaming normative fields, changing identity or
tier rules. Additive (minor): new optional comment fields, new sidecar
files, new directories. **Covenant binding:** app v1.0 ships if and only if
`bundle_format` 1.0 is frozen, and every 1.x ingester MUST open every ≤1.x
Bundle.

## 9. Ingestion & the ledger

The ingester is the only writer. Importers produce Bundles (or Bundle
fragments); they never see the store. The ledger is core-private and never
exported: it is to import what the op-log is to sync — the memory that turns
a stateless importer into an idempotent process. Sync, not the Bundle, is
the machine-migration path.

### 9.1 Identity resolution, in order

1. Identity comment `id` present → use it.
2. Else `src` provenance present → ledger lookup on `(importer,
source_key)`; hit → reuse UUID.
3. Else miss → mint and record — under the seeding rule: a _stable_ foreign
   id seeds structurally, `UUIDv5(ns_importer, stable_id)`; anything else
   mints randomly and relies on the ledger. Content NEVER enters a UUID
   seed — content-seeded identity is identity inferred from content (§9.4):
   it false-merges byte-identical blocks (empty bullets, repeated TODOs) and
   breaks on source edits, exactly the case re-import exists for. Content
   may only inform the _source key_ that memory is consulted with.
   Determinism by computation (UUIDv5) is an optimization; determinism by
   memory (the ledger) is the guarantee.
4. Else (no id, no provenance) → content-hash match against ledger;
   content-hash matching applies only to identity-less input — recovery for
   stripped comments and unstable importer keys, scoped per-importer within
   the ledger. **It MUST NOT unify entities that each carry explicit
   identity (§9.4).**

**Importer-minted Charts:** `ChartID`s, `FeatureID`s, and `GlyphID`s mint
randomly on first sight and resolve by ledger thereafter. No seeded
`Base36ID` minting exists, deliberately: at 36⁴ the birthday bound sits near
~1,500 values for even odds of collision, and deterministic minting cannot
collision-retry without breaking its own determinism — it would convert a
survivable retry into a silent identity merge. Random + memory is the only
safe determinism in a small ID space.

### 9.2 Ledger schema

The ledger is a materialized view — `bedrock`'s
`experimental_materialized_views` — over the namespaced `extra` keys:
`so.waypoints.importer.{name}` carrying `{id, content-hash, last-updated}`
on each imported entity. The view indexes
`(importer, source_key) → entity_id, content_hash, last_updated`, unique per
importer, so lookup-before-mint is an O(1) probe — never a graph load.
Idempotency as a maintained invariant, with the invariant's one superpower
over a theorem: heuristics. Unstable-key matching happens at most once per
entity, gets recorded, and every subsequent run is deterministic by lookup.
Plugins never define views — they declare their namespace key to the
importer API, and the core builds and queries the view.

_(See the local carriage note at the top: FINDINGS V1 + design.md D4 replace
the MV mechanism with an ordinary core-owned table for Phase 2; the
namespaced `extra` remains authoritative and the ledger remains rebuildable
from it.)_

**◆ (0.4) `extra` schema + plugin surface: field shapes per namespace, the
view-declaration API, write permissions. Deferred by design — next
discussion.**

### 9.3 Re-import semantics (decided once, inherited by every importer)

- Same key, changed hash → update in place.
- Same key, same hash → skip.
- Key present in ledger, absent from this run → tombstone rule:
  flag-for-review.
- Ledger uniqueness violation → defined conflict resolution with a report,
  never per-importer folklore.

### 9.4 Identity is declared, not inferred

Detaching a shared `Mark` — the unsync operation — mints a new `MarkID`
whose content is initially identical to its source.
Different-`MarkID`-same-hash is therefore a valid, expected state, not a
collision. Two rules follow:

1. Content-hash matching is confined to identity-less input (§9.1.4) and
   MUST NOT merge explicitly-identified entities.
2. Store-wide hash-based deduplication MAY only ever be advisory — suggest,
   never auto-merge. Content similarity is evidence for recovery, never
   grounds for unification.

## 10. Conformance

- **Roundtrip property tests:** seeded from the `getting-started` collection
  (which MUST remain a valid Bundle at all times — it already "mirrors the
  export format" by design). Property: `ingest(export(store)) == store`.
  Equality is over content, structure, and identity: a re-ingested store is
  a new device with fresh clocks and the same knowledge.
- **The 1980 test (CI):** strip all comments from every projection file,
  render with a vanilla CommonMark renderer, assert no Waypoints residue and
  no rendering errors. This mechanizes [L].
- **Importer conformance:** an importer is conformant iff its output Bundle
  validates (manifest + schemas + this grammar) and ingests without report.
  Golden fixtures per importer double as the migration matrix's fidelity
  column — the what-survives table cites tests, not assertions.

## 11. What this spec deliberately does not do

No sync semantics (the Bundle is a snapshot, not an op log). No Beacons
(reserved, §2). No encryption (Bundles are plaintext by definition — the
export _is_ the escape hatch; encrypt at rest with your filesystem). No
prescribed directory for user organization inside `library/` beyond the
Chart grouping (§3). And no foreign-roundtrip enablement: the Bundle
serializes a Waypoints store in stable, legible, file-based form — full
stop. It embeds no foreign-platform metadata to make "Platform" → Waypoints
→ "Platform" isomorphic; the ID guarantees already hand any would-be
importer-to-elsewhere everything they need. Embedding more would make every
export an eternal compatibility promise to someone else's schema — an
ungodly maintenance precedent, refused here in writing.

## 12. ◆ Decision ledger (running)

- [x] Filename policy — UUID-keyed (John's ruling) — §3
- [x] Frontmatter = flattened root Mark incl. created/modified — confirmed —
      §4.1
- [x] Timestamps — wall-clock in frontmatter; HLCs excluded from the
      Bundle — §4.1
- [x] Rich-text Readings — YAML block scalars, single shared parser path —
      §4.1
- [x] Divergent duplicates — single-render invariant + loud refusal on
      Bundle surgery — §4.3
- [x] ◆ Comment pairing: always-pair containers, never-pair leaves
      (proposed) — §4.3
- [x] Namespace — `wp:`, exposure therapy accepted — §4.3
- [x] ◆ Asset hash — inherit Outpost's (ruled); document which algorithm —
      §7
- [x] Content-hash fallback — identity-less input only, per-importer scope —
      §9.1
- [x] Tombstone rule — flag-for-review (John's ruling) — §9.3
- [x] Stripped-identity flaw — specified as the detach rule — §9.4
- [x] Roundtrip equality — content/structure/identity; HLC clause
      dissolved — §10
- [x] ◆ Layout: flat `landmarks/` (previous) vs. Chart-grouped
      `charts/<id>/` — resolved as `library/<ChartID>/` — §3
- [x] Ledger export — not exported; core machinery — §3/§9
- [x] Feature/Chart/Reading ID shapes — verified against `crates/domain`
      (John's catch): base-36, `builtin.*`, no ReadingID — §2/§4.1/§5
- [x] Importer-minted Charts — random mint + ledger; no seeded base-36
      (birthday bound) — §9
- [x] [F] rescoped to Waypoints stores; [I] named as the third guarantee —
      §1 (John's ruling)
- [x] `extra` carriage — residue-per-entity: `marks.jsonl`; Charts inside
      `_schema.json`; edges unified into `terrain.jsonl` (John's collapse:
      `Custom` stores its data in `extra` anyway) — §3/§6
- [x] Ledger = materialized view over `extra`; plugins declare namespaces,
      core builds and queries — §9.2
- [x] No content in UUID seeds; content informs source keys only — §9.1
- [x] Stale paths fixed after `library/` ruling — §5, §7, §11
- [x] TerrainID carriage — never: natural-key correspondence, ordinals not
      order-strings, unconditional regeneration — §6
- [ ] ◆ `extra` schema + plugin view-declaration API — §9 (0.4, deferred by
      design)
