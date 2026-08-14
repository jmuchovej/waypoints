## Context

See proposal.md — Why. The spec deltas in `specs/` are the normative
statement of what this change implements; `projection-spec.md` in this
change directory (Draft 0.3, pulled from Notion 2026-08-18, redline
resolved) is the historical drafting record. `FINDINGS.md` (repo root,
2026-08-18) is the verification record this design is constrained by. The constraints that shape everything below:

- No markdown serializer exists in the workspace; the only parser is the
  legacy `importer` line-scanner with a 3-key frontmatter (`FINDINGS.md` V2).
- Turso 0.7.0 materialized views are not maintained after declaration and
  bedrock's own API documents them as recompute-only (V1) — anything
  ledger-shaped must not assume them.
- `Chart` lacks `extra` (V5); `Mark`/`Terrain` have it as `Option<Json>`.
- `op-log-completeness` (in-flight change) owns the `Landmark.children`
  removal and retains `RichReference`; this change layers one delta on top
  (un-export) and must not duplicate that scope.
- House rules: persistence tests run against the real engine, never
  string-asserted SQL; every task lands hook-green; treefmt owns formatting.

Decisions marked **(proposed — ratify at review)** are calls this design makes
where `projection-spec.md` is silent; John's review of this change is the
ratification gate. Everything else implements a ruling that already exists.

## Goals / Non-Goals

**Goals:**

- `export(store) → Bundle` and `ingest(bundle) → store` with the conformance
  suite proving [L], [F], and the refusal semantics.
- Byte-stable exporter output for a given store.
- The four alignment changes, each landable as an independent commit.
- The seed corpus reborn as `getting-started` in Bundle grammar.

**Non-Goals (design-level, beyond the proposal's):**

- No streaming/incremental export — Phase 1 exports whole stores; stores are
  small. Revisit only with measurements.
- No Bundle encryption, compression, or archive container — it is a plain
  directory, per spec §11.
- No public API stability promise on the `transload` crate — the Bundle
  format is versioned by `manifest.bundle_format`; the Rust surface is not
  frozen by this change.

## Decisions

### D1 — Crate topology: `transload` sits below `store` (2026-08-18)

`crates/transload` depends on `domain` + `bedrock` only. The ingester writes
Marks/Terrains/Charts through `bedrock::Database` generics exactly as
`store::collections::demo` does today; it needs no `store` API. `store` gains
a dependency on `transload` for seeding (ingesting the `getting-started`
Bundle), replacing its optional `importer` dependency.

Consequence for tests: grammar and property tests live in `transload` with
self-contained Chart fixtures; the corpus roundtrip test lives in `store`
(which owns `test_utils::temp_db()`, the builtin Charts, and the corpus).
This avoids a dev-dependency cycle rather than leaning on cargo tolerating
one.

- **Rejected — export/ingest inside `store`:** bloats the crate whose job is
  persistence primitives, and drags collection machinery into every grammar
  test.
- **Rejected — extend `crates/importer`:** wrong name for a bidirectional
  carrier, and Phase 3 needs that crate free to become the plugin-facing
  surface.
- **Rejected — `transload` above `store`:** cycles the moment seeding
  ingests the corpus.

### D2 — Frontmatter: hand-rolled deterministic emitter, `saphyr` parser (2026-08-18)

Emit side is a purpose-built writer: §4.1 requires advisory `#` comments
(feature names, ref labels), normalized block-scalar chomping, and
byte-stability — no serde YAML library emits comments, and determinism is
easier to own than to configure. Parse side uses `saphyr` (yaml-rust2
lineage): frontmatter keys are dynamic (`FeatureID`s), so a node-tree parser
fits where serde derive cannot; comments are advisory and never trusted on
ingest, so parse-side comment loss is correct behavior, and hand-edited
Bundles (surgery is a supported refusal path, not a crash path) get real YAML
tolerance instead of a brittle subset scanner.

- **Rejected — `serde_norway`/`serde-saphyr`:** serde's model wants typed
  structs; readings are FeatureID-keyed dynamic maps, and neither emits
  comments.
- **Rejected — extending the legacy line-scanner:** V2 measured its ceiling
  (three keys, no block scalars, no nesting).
- **Risk noted:** `saphyr` is pre-1.0 (0.0.x). It is confined behind one
  `frontmatter` module boundary so it can be swapped without touching
  grammar code.

### D3 — `_schema.json`: Draft-07 face + canonical Chart under `x-waypoints` (2026-08-18) (proposed — ratify at review)

Draft-07 keywords (generated from `Feature`/`FeatureKind`: types, `required`,
enum values for selects, descriptions) make the schema genuinely usable by
1980-tooling and by the ingester's instance validation. But Draft-07 cannot
carry FeatureKind's full richness (reference display modes, glyph colors,
grouped selects), and [F] demands lossless Charts — so the canonical
serialized Chart (name, icon, description, features, `extra`) rides in a
top-level `x-waypoints` member. Unknown keywords are Draft-07-legal; the CI
meta-schema check stays honest. Ingest reads `x-waypoints` for fidelity and
uses the Draft-07 face for validating frontmatter instances. The Chart's
`extra` therefore lives inside `x-waypoints` rather than as a bare top-level
key — one reserved member instead of N.

Validation dependency: the `jsonschema` crate (Draft-07 support) for both the
CI meta-schema check and ingest-side instance validation.

- **Rejected — `schemars`:** derives schemas from Rust types; Charts are
  runtime data.
- **Rejected — Draft-07-only:** silently lossy for FeatureKind — an [F]
  violation by construction.

### D4 — The ledger is an ordinary core-owned table (binding for Phase 2; recorded here, not built) (2026-08-18)

Five fixed columns — `importer`, `source_key`, `entity_id`, `content_hash`,
`last_updated` — with a composite `UNIQUE(importer, source_key)`, maintained
in the same transaction as entity writes. The shape is importer-invariant
because §9.1/§9.3 are importer-agnostic algorithms; everything source-shaped
stays in the entity's namespaced `extra` (already schema-less JSON), which
remains authoritative — the table is a rebuildable index over it, preserving
"re-ingesting a Bundle rebuilds the ledger". Plugins declare exactly one
thing: their namespace (already in the ABI manifest).

- **Rejected — materialized view (the spec's §9.2 mechanism):** FINDINGS V1 —
  the pinned engine never maintains MVs post-declare, and a unique index on
  one enforces only at build time.
- **Rejected — per-plugin tables / plugin-declared DDL:** nothing per-plugin
  needs columns, and plugin DDL breaches the zero-host-imports sandbox story.
- Content hashes are BLAKE3-256, bare lowercase hex — no `b3:` prefix
  anywhere (filenames or values); colons are invalid in Windows filenames and
  one convention beats two.

### D5 — Timestamps: hook stamps on `Update` only; export as ISO-8601 `Z` (2026-08-18)

`touch` discriminates on the `WriteOp` it already receives: `Update` stamps,
insert-class writes (`insert`, `insert_all`, `upsert`) preserve caller values.
That is what lets ingest write `created`/`updated` as domain data. Frontmatter
renders `NaiveDateTime` (naive UTC, millisecond-truncated) as ISO-8601 with an
explicit `Z` and millisecond precision; DB columns stay `timestamp`, so engine
sort/search is untouched. Roundtrip equality includes both timestamps.

### D6 — Soft-deleted entities do not export (2026-08-18) (proposed — ratify at review)

The exporter excludes rows with `deleted_at` set, for Marks and Terrains
alike. A Bundle is a freeze-frame of knowledge; deletion in a freeze-frame is
absence. Exporting tombstone markers would either resurrect content on ingest
or demand un-legible corpse files — and sync, not the Bundle, is the carrier
of deletion history. Roundtrip equality quantifies over live entities.

- **Rejected — exporting tombstones with a marker:** [L] violation, and it
  smuggles op-log semantics into a format that rules them out (§4.1).

### D7 — Inline mentions are wiki links (2026-08-18, John's ruling)

An inline `Mention` projects as a `[[…]]` wiki link — the dialect's
internal-reference form, distinct from external `[text](url)` links. The
interior identifies the target losslessly (a `MarkID`); labels are advisory
and regenerated, separated by a pipe (`[[target|label]]`, label optional).
The target scheme (bare id vs. relative Landmark path) is pinned by
fixtures before format freeze.
A mention that must carry extra metadata (a non-default `RichReference`
display mode) may escalate to an inline component — the only reserved
use of inline components in the dialect. Per the §4.4 carrier rule the
target is content semantics (readers see a link), so it rides in link
syntax, not comments.

- **Rejected — raw `@uuid` text (current demo grammar):** UUID soup in 1980.
- **Rejected — identity comments per mention:** §4.4 puts reader-visible
  semantics in attributes/syntax, and inline comments would shred legibility.
- **Rejected — `[label](wp:<MarkID>)` / relative CommonMark links (earlier
  draft of this decision):** superseded — internal and external references
  should not share a spelling.

### D8 — Body grammar: pulldown-cmark runtime options + a transload-owned inline layer (2026-08-18)

Parse uses `pulldown-cmark` with runtime `Options` for tables, strikethrough,
and math (no cargo-feature change). Waypoints-specific inlines that
CommonMark lacks (superscript, subscript, spoiler, mentions) are handled by a
small inline lexer layered over the event stream on parse and by the
serializer on emit — one module, both directions, which is what "single
shared path" means concretely. The dialect grammar module (inline lexer +
block serializer) lives in `domain` behind a `syntax` cargo feature (John's
ruling, 2026-08-19): the dialect is defined where `Content`/`TextMarkKind`
are, and its reference doc rides at `crates/domain/docs/`. `transload`
enables `domain/syntax`; every other dependent pays nothing —
`pulldown-cmark` becomes an optional `domain` dependency gated by the
feature. Bundle-level grammar (frontmatter, identity comments, manifest)
stays in `transload`. Inline marks canonicalize to the span form
(`[text]{attrs}` with bare mark tokens); the character spellings are
shortcuts the emitter prefers wherever one exists, and only math and
mentions are irreducible to spans (John's framing, 2026-08-19). Block themes
are the coupled `theme-{color}` token, matching `BlockTheme`'s single color;
independent ink/surface is inline-only (`text-`/`bg-` span tokens — the
emitted keys keep the evergreen Tailwind names; the domain layer names the
channels ink/surface internally). The dialect is a strict subset (single
canonical spelling per construct — `-` bullets only, `+` auto-numbered
enums with `N.` for explicit numbers, two-space canonical indent, ATX
headings only, fenced code only, `---` dividers only; John's rulings
2026-08-19/20); well-formedness is `emit(parse(doc)) == doc`, enforced by
canonical re-serialization equality rather than a hand-maintained
validator, with one deliberate leniency: list structure parses Typst-style
by relative indentation and normalizes to the canonical indent on re-emit.
Explicit enum numbers need a field `EnumItem` does not yet carry — marked
(pending) in the dialect doc until that domain change lands. The dialect's
emphasis remapping (`*bold*` = bold, `_italic_` = italic, `__underline__` =
underline — a deliberate CommonMark departure, chosen for how it reads)
means the inline layer resolves pulldown's merged `Emphasis`/`Strong` events
by delimiter character via source spans (`OffsetIter`): `*` → bold, `_` →
italic, `__` → underline; `**` never appears in emitted files. Callouts are
block components (`::callout{icon="…" color="…"}` … `::`, John's ruling
2026-08-18 — not GFM-style alerts), handled by a small block-component layer
alongside the inline lexer; block components are the dialect's
structured-block extension point. The component/attribute grammar is
Comark/MDC-inspired, but the dialect doc is normative — no external dialect
is tracked or named as authority. The serializer's escaping discipline (what
must be backslash-escaped so the parse direction is exact) is fixture-tested
per Content variant.

### D9 — Ordinals from fractional keys, and back (2026-08-18)

Export: parallel `(src, dst, kind)` edges take `order` = occurrence index,
ordered by the runtime fractional key (Child edges) or reading-list position
(Tier-2). Ingest: fractional keys regenerate via
`domain::fractional_index::generate_n` (the public re-export — the `_shared`
parent module is `pub(crate)`) over each sibling sequence.
Order _strings_ never appear in a Bundle; `GroupedSelect`'s `HashMap` (and
every other unordered iteration) is sorted at the serialization boundary —
byte-stability is a property of the writer, not a hope about hash seeds.

### D10 — Builtin Charts export; only custom Charts ingest as writes (2026-08-18)

Every Chart referenced by an exported Landmark emits a `_schema.json`,
builtins included — the Bundle stays self-describing without the app. On
ingest, `builtin.*` bindings resolve to the app's own builtins (the Bundle's
copy validates instances but never overwrites code-defined Charts); custom
Charts upsert from `x-waypoints`. A Bundle whose builtin schema requires
features the app's builtin lacks produces a per-Chart entry in the conflict
report — refusal, not silent narrowing.

### D11 — Manifest: `bundle_format: "0.1.0"` pre-freeze; territory as caller metadata (2026-08-18) (proposed — ratify at review)

The covenant freezes `1.0.0` at app v1.0; until then the exporter writes
`0.1.0` so no Bundle in the wild claims the frozen format. Territory identity
is an `ExportMeta` argument supplied by the caller (cartograph owns identity;
`transload` should not depend on outpost for one string) and is optional in
0.x. Entity counts and exporter version are computed; per-file checksums stay
OPTIONAL and unimplemented in 0.1.0.

### D12 — Assets: reserved, not implemented (2026-08-18)

No asset subsystem exists in-tree (`FeatureKind` has no file variant — file
features are currently out of scope; `Reading::File(String)` is an inert
scalar that exports verbatim). The exporter creates no `assets/` directory;
the ingester treats an unexpected one as unknown-content refusal per the
malformed-Bundle rules. When an asset store exists, §7 applies (BLAKE3-256
bare-hex filenames).

### D13 — Refusals are typed data (2026-08-18)

`ingest` returns a structured error carrying per-item collections — divergent
same-`id` duplicates as a per-UUID conflict list (each with both file paths),
orphan annotative terrain lines as a per-line malformed-Bundle list with
source locations. No partial ingest on refusal: the whole run is one
transaction, so a refused Bundle leaves the store untouched. This struct is
deliberately the seed of Phase 2's import report, but nothing here depends on
that future.

### D14 — The legacy `importer` crate goes dormant, explicitly (2026-08-18)

After seeding switches to Bundle ingest, `crates/importer` has zero
consumers. It is left in-tree untouched — it is the natural seed of Phase 3's
first plugin (identity-less markdown is exactly §9.1.4's recovery case) — and
its dormancy is recorded here rather than discovered later. Removing it now
would be scope creep; retrofitting it now would be work with no consumer.

## Risks / Trade-offs

- [`saphyr` pre-1.0 churn] → confined to the `frontmatter` module; grammar
  and conformance tests are parser-agnostic, so a swap is mechanical.
- [Byte-stability is easy to claim, hard to keep] → the roundtrip suite
  asserts `export(ingest(export(s))) == export(s)` bytes, and golden fixtures
  pin the corpus; any emitter change that shifts bytes fails visibly.
- [`op-log-completeness` sequencing] → only the RichReference un-export task
  blocks on it; every other task is independent. If it lands mid-change, the
  un-export is a one-commit follow-up.
- [Corpus hand-audit is a manual gate] → the regenerated `getting-started`
  corpus ships only after John reads it; the one-off conversion flow is
  scripted and repeatable so the audit can iterate.
- [`upsert` no longer refreshing `updated_at`] → behavior change with no
  in-tree dependents today; called out in the spec delta so a future caller
  reads the contract, not the old habit.
- [Draft-07 `x-waypoints` double-encoding] → the Draft-07 face and the
  canonical Chart could drift; the conformance suite generates the face _from_
  the canonical form at export and cross-validates on ingest, so drift is a
  test failure, not a latent bug.

## Migration Plan

1. Alignment commits land first, each hook-green and independently
   revertible: `Chart.extra` (bedrock's spec-diff adds the nullable `jsonb`
   column on next open — no hand-written migration, matching how every
   collection change ships), the `touch` discrimination, the `json_path`
   namespace support (serde-defaulted fields; stored view specs unaffected).
2. The `transload` crate builds up behind its own tests; nothing user-facing
   moves until the corpus swap.
3. Corpus conversion is a one-off, scripted: temp DB → seed builtins → legacy
   demo seed (the legacy importer's last act) → `export` → hand-audit →
   commit as `src/collections/getting-started/` → seeding flips to Bundle
   ingest → legacy corpus and `demo.rs` removed in the same commit.
4. Rollback: alignment items are additive or no-op-by-default; the corpus
   swap is one commit and reverts cleanly (the legacy corpus lives in git
   history).

## Open Questions

- Whether the `import` domain-registry entry gets renamed or drained into
  `transload` — John's `openspec/config.yaml` call, naturally settled at
  archive time; nothing in this change reads the registry.
- ~~Exact spellings for superscript, subscript, and spoiler~~ — resolved
  2026-08-18 by John's rulings, recorded in D7/D8: `^`/`_` with Typst/LaTeX
  brace rules, `||spoiler||`, `==highlight==`, Tailwind-parallel utility
  tokens for color/size/family. Remaining open: the `comment` mark spelling
  and the wiki-link interior convention — fixtures pin both before format
  freeze.
