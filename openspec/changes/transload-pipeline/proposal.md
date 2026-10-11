## Why

Waypoints' data covenant promises an exit: every store exports to a directory of
human-legible markdown that reads as text, proper, in any editor in 1980 or
2180, and that Waypoints itself can re-ingest without loss. The design is
settled — Draft 0.3 of the projection spec, resolved through ~22 rulings and
since split into its targeted forms (a markdown-dialect reference and an
export-format contract, each standing on its own; `projection-spec.md` in this
change directory is the historical drafting record) — but none of it is
implemented: the repo has **no exporter of any kind** (no markdown serializer
exists in any crate), an importer that runs one direction only, and a seed
corpus that predates the export format. The verification spike (`FINDINGS.md`,
repo root, 2026-08-18) measured the distance item by item and surfaced four
repo-side facts the spec's rulings now incorporate: turso's materialized views
cannot serve as the import ledger, `Chart.extra` was dropped and must return,
`Landmark.children` is dead (removal already owned by `op-log-completeness`),
and the `touch` hook overwrites the very timestamps the Bundle must preserve.

This change implements the Bundle format and the `export`/`ingest` pair —
Phase 1 of the transload pipeline — plus the four small domain/storage
alignments that Phase 1 is built on. The importer plugin system (Phases 2–3)
is out of scope.

**Authority order:** this change's spec deltas (`specs/`) are normative for
what Phase 1 implements — they state the export contract in full and cite
nothing this change itself creates. `projection-spec.md` (this directory,
Draft 0.3) is the historical drafting record and the sole carrier of the
importer-side material (§9 ledger semantics) until `importer-abi` comes
down; where the two disagree, the deltas win. `FINDINGS.md` is the
verification record. The Notion page retains the redline history but no
longer needs consulting for this change.

## What Changes

**Alignment (pre-exporter, each independently landable):**

- `Chart` regains `extra: Option<Json>`, restoring `extra` parity across all
  three residue-bearing entities (Mark, Terrain, Chart). Feature-level extra
  lives _inside_ the Chart's `extra` keyed by `FeatureID` — Features have no
  table of their own — with keys matching `^[0-9a-z]{4}$` reserved as feature
  slots. Additive: bedrock's spec-diff migration adds the nullable `jsonb`
  column on next open; generated TS bindings gain an optional field.
- The `Mark` before-save hook stamps `updated_at` on `WriteOp::Update` **only**.
  Insert-class writes (`insert`, `insert_all`, `upsert`) preserve caller-provided
  timestamps, which is what lets ingest write a Bundle's `modified` time as domain
  data instead of losing it to the wall clock. **Behavioral change:** an `upsert`
  of an existing row no longer refreshes `updated_at` implicitly; no in-tree
  caller depends on that (live edits go through `update`).
- bedrock's JSON-path lowering learns namespaced keys: `ViewColumn` and
  `ViewScope` gain `ns: Option<String>` (serde-defaulted; stored specs stay
  compatible), and `ddl::json_path` emits the namespace as a first path element
  that is always quoted and never split on dots. Existing single-segment dotted
  paths keep today's split behavior byte-for-byte.
- `RichReference` is un-exported: removed from `domain`'s public re-exports and
  typegen registration; the module and type stay in-tree (soft deletion — the
  display-mode concept is parked for a future Terrain/Chart embedding).
  **Depends on `op-log-completeness`**, which deletes `Landmark.children` and
  explicitly retains `RichReference` _exported_; this change tightens that to
  retained-but-private. **BREAKING** only for the generated TS surface; no
  frontend source imports it.

**The transload pipeline (Phase 1 proper):**

- A new `transload` crate owning Bundle grammar: the Landmark file (YAML
  frontmatter with `FeatureID` reading keys and advisory human-name comments;
  body as the Tier-1 child-edge tree with `wp:mark` identity comments —
  containers always paired open/close, leaves never), the sidecars
  (`terrain.jsonl` carrying constitutive Tier-3 records and annotative
  extra-bearing Tier-1/2 records; `marks.jsonl` carrying Mark extra),
  `_schema.json` per Chart (Draft-07, embedding the Chart's `extra`), and
  `manifest.json`. Exporter output is byte-stable for a given store.
- `export(store) → Bundle` and `ingest(bundle) → store` as a pair. Identity
  resolution on ingest follows projection-spec §9.1; `TerrainID`s, order
  strings, and clocks regenerate; Bundle edge addressing is by natural key
  `(src, dst, kind, ordinal)` and no `TerrainID` ever appears in a Bundle.
- One shared segments↔markdown path used by both directions — serializer and
  parser live in the `transload` crate and both the exporter and ingester run
  through them. The serializer is written new (none exists anywhere in the
  workspace). The legacy `importer` crate is _not_ retrofitted here: once
  seeding ingests the Bundle-form corpus it has zero consumers, and it stays
  in-tree untouched as the Phase-3 plugin precursor — flagged as intentionally
  dormant rather than silently kept.
- Loud refusals, never silent picks: divergent same-`id` duplicates (Bundle
  surgery) → per-UUID conflict report; an annotative terrain line matching no
  projected edge → malformed-Bundle refusal.
- Exporter dispatch over `TerrainKind` is a non-wildcard match; adding a
  variant without a tier assignment fails compilation (enforced by a
  `trybuild` compile-fail test).
- Conformance suite: `ingest(export(store)) == store` property tests (equality
  over content, structure, identity); the 1980 test (strip comments → zero
  machine residue, still-legible text); refusal tests; every exported
  `_schema.json` validates against the Draft-07 meta-schema in CI.
- The seed corpus becomes the `getting-started` collection in Bundle grammar:
  the current `demo` corpus is ingested once, exported through the new
  exporter, hand-audited, and committed under the name the website already
  advertises. The collection then doubles as the roundtrip fixture the
  conformance suite seeds from.

## Capabilities

### New Capabilities

- `transload`: the Bundle format and its three guarantees — [L] legibility,
  [F] fidelity (`ingest(export(store)) == store`, quantified over Waypoints
  stores), [I] idempotency — plus the export/ingest contracts, edge tiers,
  identity rules, and refusal semantics. **Domain-registry note:** no
  registered domain fits — `import` covers foreign-source parsing
  (`importer` crate, one direction), while this capability is the
  bidirectional Bundle carrier and its identity rules. Minting `transload`
  openly rather than silently; the registry in `openspec/config.yaml` is
  John's to amend, and `import`'s current content is expected to drain into
  this capability when the legacy parser retrofits onto the shared path.
- `data-model`: seeded with the invariants this change owns — `extra` as
  domain data on Chart/Mark/Terrain (feature-keyed convention included) and
  Mark timestamp semantics (`created_at` caller-settable; `updated_at`
  stamped only on update-class writes). Not an exhaustive description of the
  model. **Naming-drift note:** `op-log-completeness` seeds a capability named
  `domain` for the same registry area the config calls `data-model`; this
  change follows the registry and flags the collision for reconciliation at
  archive time rather than adding a third opinion.
- `storage`: seeded with one requirement — JSON-path lowering for view
  columns/scopes supports a namespaced top-level key, always quoted, never
  split; single-segment dotted paths retain split semantics.

### Modified Capabilities

None. `openspec/specs/` is empty — specs emerge from differentials; this
change creates the three above rather than amending existing ones.

## Non-goals

- **No ledger, no import report.** Phase 2. The _decision_ that the ledger is
  an ordinary core-owned table transactionally maintained by the ingester —
  not a materialized view, per `FINDINGS.md` V1 — is recorded in this change's
  design.md as binding context for that phase, but nothing here builds it.
- **No importer ABI, no WASM host, no SDK, no Logseq importer.** Phase 3. No
  Fragment (`src`-keyed) parsing lands here; this change implements the
  export-Bundle (`id`-keyed) half of the grammar split only.
- **No Beacons** (the `beacons/` directory name stays reserved), **no
  sync/Outpost changes**, **no TS extension tier**.
- **No edits to the Notion specs** and no resolution of any open ◆ decision
  marker (`extra` schema / plugin surface stays deferred).
- **No seeded base-36 minting** under any justification, and no content ever
  enters a UUID seed (both are projection-spec rulings this change enforces,
  not revisits).
- **No foreign-platform metadata** in Bundles; no store-wide content-hash
  deduplication.
- **No frontend/UI surface.** Export/ingest land as crate APIs with
  conformance tests; wiring them to Tauri commands or app UI is a later
  change.

## Impact

**Code**

- `crates/domain` — `Chart.extra` field + builder mutator + specta annotation;
  `touch` hook op-discrimination in `mark.rs`; `RichReference` un-export in
  `lib.rs` and `bin/typegen.rs` (after `op-log-completeness` lands its removal
  of `children`).
- `crates/bedrock-core` — `ViewColumn.ns` / `ViewScope.ns`.
- `crates/bedrock` — `ddl::json_path` signature and quoting; a view test over
  a namespaced key against the real engine.
- `crates/transload` (new) — Bundle grammar, exporter, ingester, conformance
  fixtures. Depends on `domain` + `bedrock` only; `store` gains a dependency
  on `transload` for seeding (design.md, D1 — the reverse direction would
  cycle). Sits beside `cartograph`, which does not change.
- `crates/importer` — untouched, but loses its last consumer when seeding
  moves to Bundle ingest; left dormant pending Phase 3 (see design.md).
- `crates/store` — the `demo` corpus module (`src/collections/demo.rs` plus
  the `.md` corpus under `src/collections/data/`) is replaced by
  `src/collections/getting-started/` in Bundle grammar; seeding updated to
  ingest the Bundle-form corpus.
- Generated TS bindings regenerate (`Chart.extra` appears; `RichReference`
  disappears).

**Dependencies**

- A YAML strategy for frontmatter (no YAML crate is currently in the
  workspace, and advisory comments require emit-side control no serde YAML
  library provides) — decided in design.md.
- A Draft-07 JSON Schema validator (meta-schema CI check and ingest-side
  schema validation) — new dependency, decided in design.md.
- `trybuild` is already a workspace dev-dependency; `pulldown-cmark` remains
  the parse-side base.

**Systems**

- CI gains the 1980 test and the meta-schema check alongside the conformance
  suite (crate-level tests; no workflow rewrite).
- The website's public claim (`website/content/about.md`) that a committed
  `getting-started` collection mirrors the export format becomes true. Note
  the page's link points at `crates/store/collections/getting-started` while
  the corpus lands at `crates/store/src/collections/getting-started/` — the
  link path needs a one-line follow-up fix; the page itself is not edited
  here.
