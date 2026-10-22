Every task lands with hooks green (clippy, cargo-check, cargo-test, treefmt);
never hand-format. Persistence behavior is tested against the real engine —
no string-asserted SQL anywhere, including the json_path compat task (assert
projected rows, not emitted DDL text). Groups 1–3 have no ordering between
their subtasks except where noted; groups 4–7 build on 3.

## 1. Alignment — domain & bedrock (independently landable commits)

- [x] 1.1 Add `Chart.extra: Option<Json>` in `crates/domain/src/chart/chart.rs`
      mirroring `Mark.extra` (builder mutator, specta `Unknown` annotation);
      real-engine test: save a Chart with namespaced + feature-slot-keyed
      `extra`, read back identical, and confirm bedrock's spec-diff adds the
      nullable column to a database created before the field existed.
- [x] 1.2 Discriminate `touch` on its `WriteOp` in
      `crates/domain/src/mark/mark.rs`: stamp `updated_at` on `Update` only.
      Real-engine tests: `insert_all` and `upsert` preserve explicit
      timestamps; `update` stamps.
- [x] 1.3 Add `ns: Option<String>` (serde-defaulted) to `ViewColumn` and
      `ViewScope` in `crates/bedrock-core/src/view.rs`; rework
      `ddl::json_path` to `(segments, ns)` semantics per the storage spec.
      Real-engine tests in `crates/bedrock/tests/views.rs`: a view over a
      dotted namespace key projects the right values; a legacy dotted path
      declaration projects identically to before.
- [x] 1.4 Un-export `RichReference`: remove from `crates/domain/src/lib.rs`
      re-exports and the typegen registration (now
      `tooling/xtask/src/tasks/typegen.rs`, per `lift-typegen-into-xtask`);
      regenerate TS bindings; leave the module in-tree with a doc note
      pointing at the data-model spec decision. (`op-log-completeness`
      archived 2026-09-29, so the `children` removal had already landed.)

## 2. transload crate scaffold

- [x] 2.1 Add the `syntax` cargo feature to `domain` (gates the dialect
      grammar module; `pulldown-cmark` becomes an optional dependency behind
      it). Create `crates/transload` (workspace member; deps: `domain` with
      `syntax`, `bedrock`, `saphyr`, `jsonschema`, `serde_json`, `uuid`,
      `chrono`; lints inherited). Confine `saphyr` behind a `frontmatter`
      module boundary per design D2.
- [x] 2.2 Define the crate's spine: Bundle path layout, `ExportMeta`
      (exporter version, timestamp, optional territory), and the typed
      refusal structures (per-UUID conflict list, per-line malformed list
      with source locations) per design D13.

## 3. Shared grammar modules

Each lands with bidirectional unit fixtures; 3.1–3.2 live in `domain` behind
the `syntax` feature per design D8, 3.3–3.4 in `transload`.

- [x] 3.1 Inline layer: `RichText` segments ↔ dialect inline syntax — the
      emphasis remapping, links, `[[…]]` mentions per design D7, and the
      Waypoints-only inlines per design D8 (span syntax canonical, character
      spellings as shortcuts). Escaping discipline fixture-tested
      (round-trip exactness per segment shape).
- [x] 3.2 Block layer: every `Content` variant ↔ its dialect form (paragraph,
      headings, blockquote, list/enum items, callout ↔ `::callout` block
      component, code, table, math, divider), parse side via pulldown-cmark
      runtime options plus the component/attribute layer.
- [x] 3.3 Identity comments: `wp:mark` emit + parse, container pairing /
      leaf non-pairing, unknown-field preservation through re-serialization.
- [x] 3.4 Frontmatter: deterministic emitter (FeatureID keys, advisory name
      comments, `{ref, label?}` lists, `|-` block scalars with normalized
      chomping, ISO-8601 `Z` ms timestamps) + saphyr parser that ignores
      comments and labels; round-trip fixtures including hand-mangled
      advisory content.
- [x] 3.5 Chart schema module: Draft-07 face generated from
      `Feature`/`FeatureKind` + canonical Chart (with `extra`) under
      `x-waypoints`; cross-validation of face against canonical form;
      meta-schema validation via `jsonschema`.

## 4. Exporter

- [x] 4.1 Store traversal: live entities only (`deleted_at` excluded),
      Landmarks grouped `library/<ChartID>/<MarkID>.md`, Tier-1 tree walk
      from child edges, shared Marks rendered once and byte-copied into
      every containing file.
- [x] 4.2 Edge tier dispatch as a non-wildcard `TerrainKind` match;
      `terrain.jsonl` writer — constitutive tier-3 lines with inline
      `extra`, annotative tier-1/2 lines only for extra-bearing edges,
      ordinals derived per design D9.
- [x] 4.3 `marks.jsonl`, `_schema.json` per referenced Chart (builtins
      included), `manifest.json` (`bundle_format: "0.1.0"`); byte-stability
      pass — every unordered iteration sorted at the serialization boundary.
- [x] 4.4 `trybuild` compile-fail test: a `TerrainKind` variant with no tier
      assignment fails compilation.

## 5. Ingester

- [x] 5.1 Manifest version gate (refuse-before-reading-anything), Bundle
      walk, frontmatter + body parse into a staged graph of Marks, edges,
      and Charts.
- [x] 5.2 Identity: declared `id` only; `src`-keyed identity refused as
      malformed; identical same-`id` duplicates unify, divergent ones
      accumulate the per-UUID conflict report.
- [x] 5.3 Terrain reconstruction as exactly the union of the tiers: structure
      from nesting, constitutive lines verbatim (`Reading` edges included —
      nothing is rebuilt from reading refs, per the 2026-10-21 spec
      decision); annotative `extra` attached by `(src, dst, kind, order)`
      natural key; orphan annotative lines accumulate malformed refusals.
      Strict kind parsing — never the lossy `from_storage` degradation.
- [x] 5.4 Chart handling per design D10: instance validation of readings
      against the Draft-07 face; `builtin.*` resolves to app builtins with
      per-Chart conflict entries on mismatch; custom Charts upsert from
      `x-waypoints`. (Unblocked by deriving builtin feature ids from stable
      slugs — they were re-minted on every launch.)
- [x] 5.5 Single-transaction apply: fresh `TerrainID`s, fractional order
      keys regenerated via `fractional_index::generate_n`, timestamps
      written as domain data (relies on 1.2). `WriteTx` gained typed
      `insert`/`upsert` for this.
- [x] 5.6 Refusal tests, each asserting the store is untouched afterward:
      `src` in Bundle position, divergent duplicates, orphan annotative
      line, schema-invalid reading, unsupported `bundle_format`, and a
      valid-prefix-then-invalid-tail Bundle (atomicity).

## 6. Conformance suite

- [x] 6.1 Property roundtrip in `transload` over generated stores
      (self-contained Chart fixtures spanning every Reading and Content
      variant, shared Marks, custom edges with `extra`):
      `ingest(export(s)) == s` over content/structure/identity.
      (`tests/conformance.rs`: a seeded generator, 64 stores by default,
      `TRANSLOAD_SEEDS` for a deep run — 10 000 ran clean. Its finds were
      fixed in the dialect, frontmatter, schema and ingester; the dialect
      rules they forced are recorded in `crates/domain/docs/dialect.md`.)
- [x] 6.2 Byte-stability: `export(s)` twice → byte-identical;
      `export(ingest(export(s))) == export(s)` bytes.
- [x] 6.3 The 1980 test: strip all comments from every projection file,
      assert zero machine-shaped residue (`wp:` markers and UUIDs only as
      link targets) and that every file still parses under the dialect
      grammar.
- [x] 6.4 Suite-level meta-schema check: every `_schema.json` in every
      test-exported Bundle validates against the Draft-07 meta-schema.

## 7. Corpus migration — `getting-started`

- [x] 7.1 One-off scripted conversion: temp DB → builtin seed → legacy demo
      seed (the legacy importer's last act) → `export`; stage the output for
      John's audit. **Do not commit the corpus before the audit.**
      (Staged at `target/getting-started-audit/`: 28 Landmarks over three
      builtin Charts, 264 content blocks, no warnings. The 24 legacy `@uuid`
      mentions export with their UUID as anchor text, as predicted.)
- [ ] 7.2 After audit approval: commit the corpus as
      `crates/store/src/collections/getting-started/`; flip seeding to
      Bundle ingest (`store` gains the `transload` dependency, drops
      optional `importer`); remove `demo.rs` and the legacy corpus in the
      same commit.
- [ ] 7.3 Store-level roundtrip test seeded from the `getting-started`
      collection against the real engine — the fixture the transload spec
      names.
- [ ] 7.4 Regenerate TS bindings (`Chart.extra` appears; with 1.4 landed,
      `RichReference` is gone); confirm the app compiles untouched.

## 8. Legibility pass (from the first corpus audit)

- [x] 8.1 Wiki links name their targets: Landmark titles, every holder of a
      shared, empty or id-shaped title suffixed from its id; blocks as
      `[[Page Title#prefix]]`; a target outside the Bundle keeps its id. The
      dialect gains `Links` (`emit_with`/`parse_with`); `transload` builds
      the names on both sides, ingest from an outline pass.
- [x] 8.2 Identity comments carry only the id; `marks.jsonl` carries
      `{id, created, updated, extra?}` for every Mark and is normative.
- [x] 8.3 Frontmatter: timestamps marked `# advisory`, feature names trailing
      their reading's key line.
- [x] 8.4 A blank line between sibling blocks.
- [x] 8.5 Conformance generator covers shared and empty titles, block and
      dangling targets, and labels equal to their target's title; 10 000
      seeds clean. Corpus re-staged for audit.
