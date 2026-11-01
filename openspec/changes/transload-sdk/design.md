## Context

See proposal.md — Why.
`importer-abi.md` in this directory (Draft 0.1, pulled from Notion 2026-10-29) is the drafting record; the archived `transload-pipeline` change's `projection-spec.md` §9 and its design D4 are the binding design for the host side this change does not build.
The archived `grammar-only-builds` change (2026-11-02) is the ground this one stands on, and its design's Outcome section is the evidence for every claim below about the build: `transload` has a `store` feature; `domain` and `transload` compile for `wasm32-unknown-unknown` and `wasm32-wasip1`; the data model has no minting and no clock on any wasm32 target; a probe `cdylib` linking the grammar and the data model, built for `wasm32-unknown-unknown`, has an **empty import section** at 268 092 bytes.
The spec deltas in `specs/` are normative for what lands here.
The constraints that shape everything below:

- The identity-bearing slots in `domain` are typed: `Landmark.chart` is a `ChartID`, readings are keyed by `FeatureID`, `Reading::Select` and friends carry `GlyphID`, `Reading::References` carries `Vec<MarkID>`, `FeatureKind`'s select variants carry `GlyphID` in their options, groups, and defaults, `FeatureKind::Reference` carries `Vec<ChartID>` in `allowed_types`, and `SelectOption` and `SelectGroup` each carry an `id: GlyphID`.
  `Feature` carries `description` and `required` beside `name` and `kind`.
  The one untyped slot is `InlineMark.data`, a JSON value, which a Bundle fills with a bare `MarkID` string for a mention.
- Nothing in `cartograph` or `store` creates a `TerrainKind::Reading` edge; the application creates only child edges, and Reading edges reach a store only through Bundle ingest.
  `TerrainKind::Reading` is the data model's provision for indexing reference readings as graph edges, unused so far.
- `IndexMap` equality ignores order, so "equal values serialize identically" cannot be promised for maps whose order is meaningful.
- On `wasm32-unknown-unknown` there is no host: no standard error, no clock, no entropy, and the standard library seeds its hash maps with fixed keys.
  A panic under `panic = "abort"` runs the panic hook and then traps; a trap leaves the instance's memory readable and its exports callable.
  A Rust `cdylib` for this target exports `memory` and its `no_mangle` functions and nothing else — no `_initialize`.
- The per-commit hook runs `cargo nextest run --workspace` with no feature flags.
  Under it, every workspace member's features unify, so a check about a plugin's own dependency graph can only be made inside a separate cargo invocation scoped to the plugin crate.
- Two `#[no_mangle]` exports of the same name cannot be linked into one binary, so each test binary may contain at most one macro invocation.
- `schemars` 1.x emits JSON Schema 2020-12 by default; `transload` validates with Draft-07.
- `toml` is already a workspace dependency; `schemars` is already in the lockfile; `wasmi` is not.
- A crate built outside the workspace against these crates needs the workspace lockfile and the workspace's `[patch.crates-io]` entry for `specta-macros`, or every `Type` derive in `domain` silently vanishes; a workspace member inherits both.
- House rules: every task lands hook-green; no doc files beyond rustdoc unless asked.

Every decision below was reviewed on 2026-10-30 and revised on 2026-11-02 against `grammar-only-builds`' Outcome; the ones dated those days are where the drafting record was silent or has been overtaken.

## Goals / Non-Goals

**Goals:**

- A plugin author can write, unit-test, golden-test, and build a conforming `wasm32-unknown-unknown` module with nothing but `transload` (feature `sdk`), a TOML manifest, and `cargo`.
- The Fragment grammar is exactly what projection-spec §9 consumes: a source key wherever the core mints, nothing the core owns (ids, edge identities, order strings, hashes, clocks).
- Determinism, the empty import section, and the export set are tests, run against the plugin's own dependency graph.
- The app's build is byte-for-byte unaffected.

**Non-Goals (design-level, beyond the proposal's):**

- No streaming of a single entry; `wp_entry` takes a whole entry's bytes.
  Peak memory is bounded by the largest entry, per the draft, and that is enough until measured otherwise.
- No stable Rust API promise for `transload::sdk` or `transload::harness`; the contract that freezes is the ABI and the Fragment JSON, versioned by `wp_abi` and `fragment_format`.
- No plugin-side access to the dialect parser or emitter.
  A plugin emits `Content` values; markdown text is the Bundle's carrier, not the Fragment's.
- No pluggable runtime abstraction.
  One runtime (wasmi, in the harness) and one consumer do not justify a trait; the host change that embeds a runtime is where a second implementation would appear.

## Decisions

### D1 — Fragment types mirror only the identity-bearing shapes, named after what they mirror (2026-10-29, naming 2026-10-30)

`transload::fragment` defines `SourceKey` (a non-empty string newtype) and a mirror of exactly each `domain` type that carries a minted identifier.
Every mirror is named `<Descriptor>Fragment`, so the Fragment vocabulary maps onto the data model's one for one — a Mark is a Landmark or Content, and a Fragment Mark is a `LandmarkFragment` or a `ContentFragment`:

- `LandmarkFragment { src, protochart: Option<SourceKey>, title: RichText, icon, readings: IndexMap<SourceKey, ReadingFragment>, created?, updated?, extra?, body: Vec<ContentFragment> }` (the binding is D2's).
- `ContentFragment { src, content: Content, children: Vec<ContentFragment>, created?, updated?, extra? }` — nesting is the child-edge carrier, as the Bundle body is.
- `FeatureFragment { name, description, required, kind: FeatureKindFragment }`, with `FeatureKindFragment` a mirror of `FeatureKind`: select options and groups keyed by `SourceKey` with `SelectOptionFragment`/`SelectGroupFragment` as `SelectOption`/`SelectGroup` minus their `id`, defaults as `SourceKey`, `Reference.allowed_types` as `Option<Vec<SourceKey>>` naming protocharts, every other variant's payload reused.
- `ReadingFragment`, a mirror of `Reading` with `SourceKey` where `GlyphID`/`MarkID` appear, every other variant identical.
- `TerrainFragment { src, dst, kind: TerrainKindFragment, order?, extra? }` with `TerrainKindFragment = Child | Reference | Custom` (D5 explains the missing variant), structured rather than the storage spelling because a source key may contain `::`.

`Content`, `RichText`, `InlineMark`, `Icon`, and the scalar readings are reused unchanged.
A mention's `InlineMark.data` carries the target's source key as a bare JSON string — the same slot, the same shape, a key instead of an id.
Mirrors are checked against their originals by an exhaustiveness test: a non-wildcard match from each of `Reading`, `FeatureKind`, and `TerrainKind` onto its Fragment counterpart, so a new data-model variant without one fails compilation — the same discipline the exporter's tier dispatch already enforces — with `TerrainKind::Reading` as the one documented, deliberate exclusion.

- **Rejected — generic `Content<Id>`/`Reading<Id>` in `domain`:** an importer concern reshaping the IO-free crate and every generated binding.
- **Rejected — untyped `serde_json::Value` Fragments:** no validation at the boundary, which is the one thing a typed payload buys.
- **Rejected — Bundle-grammar text Fragments (the draft's §5):** the grammar it mirrors no longer exists (bare-id comments, title links, mandatory `marks.jsonl`), it would need four new `src` forms the ingester refuses on purpose, and it type-checks nowhere for anyone.
- **Rejected — `Fragment<Descriptor>` naming (`FragmentChart`, `FragmentBlock`):** reads as a kind of Fragment rather than a Fragment of a kind, and `Block` named a thing the data model calls Content.
- **Rejected — reusing `SelectOption`/`SelectGroup` unchanged:** both carry a `GlyphID`, which a Fragment must not.

### D2 — Protocharts, not Charts; the Landmark binding is optional (2026-10-30)

A plugin does not know Waypoints' Charts and should not pretend to.
What it knows is the source's own structure — the columns of a Notion database, the properties on a Logseq page — and that is what it emits: a `Protochart { src, name, description, icon, features: IndexMap<SourceKey, FeatureFragment>, extra? }`, with `name`, `description`, and `icon` as the labels a person needs to recognize it when asked what it should become.
A `LandmarkFragment` binds to a protochart by key, or to none.
Resolving a protochart to a Chart — an existing one, a new one, or the application's page Chart as the fallback — is a decision a person makes at import, recorded by the core so the next import of the same source does not ask again.
None of that is the plugin's, so none of it is in the grammar: there is no way to name a `ChartID` from a Fragment, and the spelling of builtin Chart ids never enters it.

`None` is the binding for a Landmark the plugin has no structure for: a plain page.
Such a Landmark carries no readings, since readings are keyed by a protochart's features.
The plain-markdown fixture emits every Landmark this way and asks nothing of the mapping step.

A protochart may be completed by what later entries reveal, so a Landmark may bind to a key the run has not yet defined; the SDK checks at finalize that every bound key was defined by then (D12).

- **Rejected — plugins emit Charts and bind by `ChartID` or by key (the previous design):** a plugin cannot know which existing Chart a source structure corresponds to, and letting it name one by id tied the Fragment grammar to the spelling of builtin ids, which is about to be reconsidered.
- **Rejected — a mandatory binding with an empty protochart for plain pages:** noise in every trivial importer, and a mapping question for the user with only one sensible answer.

### D3 — Source keys are opaque strings with four scopes (2026-10-29, scoping 2026-10-30)

A `SourceKey` is any non-empty string.
The core never parses it; equality is byte equality; a plugin chooses keys stable across runs of the same source — a Logseq block UUID, a Notion page id, a file path when nothing better exists — and the draft's §1.2 rule that content never enters a UUID seed applies one level up: content may inform a key, never be one.

Keys live in four scopes, all under the plugin's manifest `namespace`, which is the ledger's `importer` column (design D4 of `transload-pipeline`):

- Mark keys — Landmark and Content Fragments share one space.
- Protochart keys — their own space.
- Feature keys — scoped by the protochart that defines them.
- Option and group keys — scoped by the feature that defines them.

A reading's key is therefore a feature key of the Landmark's protochart, and a select value is an option key of that feature; the binding, not a syntactic marker, says which space a string is read in.

- **Rejected — structured keys `{kind, value}`:** the core would gain vocabulary it never acts on.
- **Rejected — URL-shaped keys:** a plugin that has them can use them; a plugin that does not should not have to invent a scheme.
- **Rejected — one flat key space:** a Notion property named like a page, or an option named like a property, would collide for no reason.

### D4 — Three version numbers, each with one job (2026-10-29)

- `wp_abi` returns `1`: exports, signatures, buffer convention.
  Frozen.
- `FRAGMENT_FORMAT = "0.1.0"` versions the envelope — the Fragment shapes and the output and manifest structure.
  Additive minor, breaking major; the pre-freeze `0.x` convention of `bundle_format` applies.
- `domain::VERSION` says which grammar the plugin was compiled against.
  The SDK stamps it into the manifest, so a host can gate before `wp_init` the way `ingest` gates on the manifest before reading a file.

The census — which `Content`, `ReadingFragment`, `FeatureKindFragment`, and `TerrainKindFragment` variants were actually emitted — rides in `FinalizeOutput`, accumulated by the SDK across every `entry` call.
It is computed from output, never authored: an author-declared census could lie, and the Bundle's census is computed at export for the same reason.
A host reads it after the run to decide whether a newer-`domain` plugin used anything the host lacks.

Fragment and ABI types do **not** set `deny_unknown_fields` (unlike the Bundle sidecars): within a major, a reader ignores what it does not know, with one reserved name (D13).
Breaking changes bump `fragment_format`'s major, never redefine a field.
Adding a variant back to a mirrored enum is not an unknown field but a new variant; the census is what gates it.

- **Rejected — deriving Fragment versioning from `wp_abi`:** the ABI is meant to sit at major 1 indefinitely; the envelope will move more often than that.
- **Rejected — msgpack (draft §9 ◆):** golden fixtures diff as text, failures read without tooling, and the one argument for a binary encoding — inline asset bytes — is out of scope with assets.

### D5 — No reading-kind terrain lines in Fragments (2026-10-30)

`TerrainKind::Reading(chart, feature)` is the data model's provision for indexing a reference reading as a graph edge.
It is a derived index: its only legitimate source is the reading it indexes, and nothing in the application creates one — the Bundle carries the kind because the exporter's dispatch must be exhaustive, not because any store has one.
A derived index belongs to the core, not to plugin input: a plugin that could emit the edge could emit one that disagrees with its reading.
So a Fragment carries a reference reading as a `ReadingFragment::References` and nothing else; `TerrainKindFragment` has no `Reading` variant, and the exhaustiveness test names the exclusion.

Whether reading edges are ever materialized is the data model's decision, deferred; the Bundle's rule that the ingester synthesizes no edge from a reading governs Bundles, and the change that ingests Fragments decides for itself, in the open, whether to derive them.

- **Rejected — Fragments carry reading-kind lines as Bundles do (the previous design):** mirrored a vestige, asked plugins to author a derived index, and stated as its premise that "a reference reading exists twice in the store", which is false today.
- **Rejected — the SDK synthesizing the edge:** the same index, derived one layer too early.

### D6 — The sandbox is the build: `wasm32-unknown-unknown`, zero imports (2026-11-02)

A plugin targets `wasm32-unknown-unknown` and its module declares no imports.
That is not a rule a plugin follows; it is what the build produces: the data model has no minting and no clock on wasm32 (`grammar-only-builds` D4), the target's standard library has no entropy, no clock, no standard streams, and fixed hash-map keys, and the probe built from the grammar and the data model had an empty import section with no configuration at all.
A plugin that links a dependency which wants a host does not get one; it fails to instantiate under any host that offers an empty import table, which is what the harness does and what the application's runtime will do.

Determinism follows without machinery.
Same configuration and same entry sequence give the same bytes because nothing in the module can observe anything else; the harness checks it by running twice and comparing, and the module backend checks the stronger claim by instantiating with an empty `Linker`, where any import is an instantiation error that names it.

What the target takes away is standard error.
A panic under `panic = "abort"` runs the panic hook and then traps, and with no stream to write to the message would be lost.
So the SDK installs a panic hook that stores the message and location in a guest-side buffer, and an eighth export, `wp_panic() -> u64`, hands it back packed like every other output, or `0` when no panic has occurred.
A trap leaves memory readable and exports callable, so a host that catches a trap calls `wp_panic` next and folds the text into its trap report.
The fixture's `fail_on_entry` option is what makes that path testable.

This decision replaces the earlier `wasip1` design — a deterministic WASI host with a fixed clock, a seeded random source, and call counters — which `grammar-only-builds` made unnecessary: the gating it would have compensated for is done, and the five WASI imports a `wasip1` build still makes include `random_get` for hash seeding, a thing that must then be served deterministically rather than simply absent.
That list stays recorded in the archived Outcome in case a runtime ever wants `wasip1` for its standard error; nothing here chooses it.

- **Rejected — `wasm32-wasip1` with a deterministic host (the previous D6):** keeps an entropy call and a clock import in every plugin for the host to neuter, and makes the sandbox a policy of the host rather than a property of the build.
- **Rejected — a WASI allowlist:** there is nothing to allow; an empty import section is both simpler to state and strictly stronger.
- **Rejected — losing the panic message:** a bare trap is a worse report than the draft already promised, and the hook costs one static buffer and one export.
- **Rejected — writing the panic into the last output's diagnostics instead:** the output a panic interrupts is never returned, so the message needs its own channel.

### D7 — `macro_rules!`, `extern "C"`, JSON, and a boxed state handle (2026-10-29, `wp_panic` 2026-11-02)

`transload::importer!(MyImporter, manifest = "importer.toml")` expands to the eight `#[unsafe(no_mangle)] pub extern "C"` exports.
`no_mangle` keeps each symbol spelled as the ABI names it (`wp_abi`, not a mangled Rust path), so a host can look it up by name; `extern "C"` pins the calling convention to the target's C ABI, since Rust's own is unstable and a WebAssembly export is typed only in integers.
The buffer convention exists because linear memory belongs to the guest: the host cannot call the guest's allocator, so the guest exports one (`wp_alloc`/`wp_free` over `Vec<u8>` with the capacity handed across) and every byte crosses through it.
`wp_init` boxes the importer and returns `Box::into_raw` as the `u32` state inside the `ok` payload (`{"ok": {"state": <u32>}}`); `wp_entry` borrows it; `wp_finalize` takes it back.
Results serialize through `serde_json` into a fresh guest buffer whose `(ptr, len)` pack into the `u64`.
A `Diagnostic` returned by the trait is the `err` payload, so the envelope and the diagnostic have one shape; a panic is a trap, and `wp_panic` returns its message afterwards (D6).
The panic hook is installed once, on the first export call, through a `Once`; the stored message lives in a `static` the hook writes and `wp_panic` reads, and on wasm32 there is one thread, so no synchronization beyond the `Once` is needed.
The macro is `macro_rules!` — the surface is eight functions with fixed shapes and needs no syntax tree — so the SDK itself adds no proc-macro crate to a plugin's build (the schema derive of D8 is the one proc macro a plugin compiles).

- **Rejected — a proc-macro for the exports:** nothing to derive from.
- **Rejected — the trait carrying `&mut self` through `finalize`:** consuming `self` makes "the instance lives for the whole run and then does not" a type-level fact.
- **Rejected — exporting the panic buffer's address as a global instead of a function:** reading a global and then memory is two steps where one call suffices, and the function keeps every output on the same packed-return convention.

### D8 — The manifest is authored in TOML and served as JSON; the options schema is generated (2026-10-30)

A plugin's manifest has two kinds of field.
The author's — `id`, `name`, `icon`, `namespace`, and `source` (kinds, markers, extensions) — are written once, as a TOML file beside the crate's manifest, because a static description reads and diffs better as a file than as a Rust literal.
The SDK's — `abi`, `version`, `fragment_format`, `domain_version`, and `options` — are stamped: `version` from `CARGO_PKG_VERSION`, the grammar versions from the constants, `options` from the configuration type.
The macro embeds the file with `include_str!`, parses it with `toml` (parse and serde only, unknown fields refused, since an author-written file is where a misspelling hides), checks the required fields, and serves the merged document as JSON from `wp_manifest`.
Because `wp_manifest` returns no `Result` envelope, an invalid manifest traps; the harness calls `manifest()` natively first so the failure surfaces in the plugin's own tests with a message rather than in a host.

`options` is generated from `Importer::Config` by `schemars`, with `SchemaSettings::draft07()`, because a configuration type is the one place the plugin already states exactly what it accepts and a hand-written schema beside it would drift.
The SDK asserts the emitted `$schema` is Draft-07, and the harness validates it against the Draft-07 meta-schema, since `transload` validates instances with a Draft-07 validator and a 2020-12 document would pass the meta check while its newer keywords went silently unread.

- **Rejected — the manifest as a Rust value returned by `manifest()`:** the draft's shape; fine for a program, worse for a reader, and it put stampable fields in the author's hands.
- **Rejected — JSON Schema spelled in TOML:** no better than JSON Schema spelled in JSON, and the author would still be maintaining a second description of `Config`.
- **Rejected — a TOML field language the SDK expands into JSON Schema:** a DSL to maintain, for no gain over the derive.
- **Rejected — hand-authored options schema (the previous design):** two descriptions of one type.
- **Rejected — `version` in the TOML:** a second hand-maintained version drifts from the crate's.
- **Rejected — validating the TOML in `build.rs`:** a build dependency on `transload` in every plugin, or a second parser; the macro plus the harness catch the same errors earlier.

### D9 — The harness has two backends and lives behind `harness` (2026-10-29, module backend 2026-11-02)

`transload::harness` drives a plugin two ways and requires the same bytes from both.
The native backend calls the `Importer` trait directly: it validates the configuration against the generated options schema, feeds the fixture directory in the ABI's entry order, serializes every output, and compares each against a committed expected file byte for byte; determinism is the same run twice.
The module backend loads the plugin's `wasm32-unknown-unknown` artifact under `wasmi` with an empty `Linker`, so instantiation itself proves the import section empty and names any offender; it drives the lifecycle through the real exports — `wp_alloc`, the packed returns, the JSON envelopes — and asserts that every output equals the native backend's and that the export set and signatures are exactly the ABI's.
A test asserting that a panic in `wp_entry` becomes a trap, and that `wp_panic` then returns the message, runs under this backend, where a trap is a caught error rather than a dead test process.

The harness is behind `harness` so a plugin build never links `jsonschema` or `wasmi`.
It is in `transload` rather than the fixture crate so the next plugin reuses it.
It provides no host functions and needs no random source or clock; there is nothing for it to serve.

- **Rejected — inspecting the module with `wasmparser` only (an earlier design):** proved the export and import sets and nothing about whether the exports work; `wasmi` proves both and reads the sections itself.
- **Rejected — driving the module through `wasmtime` in tests:** a heavier runtime than a test needs, and the one the application will embed is the host change's choice to make; nothing here depends on which.
- **Rejected — a pluggable runtime trait:** one implementation, one consumer.
- **Rejected — an `xtask` check instead of a test:** per-commit hooks run `cargo test`, not `xtask`; a tooling surface for exercising a plugin against a real source may want an `xtask` once a runtime exists in the application, which is not this change.

### D10 — The fixture plugin is the smallest nontrivial Fragment, and its tests own the plugin-graph checks (2026-10-29, placement 2026-10-30)

`crates/transload/fixtures/plain-markdown` is a workspace member built as `cdylib` (the module) and `rlib` (the harness).
It maps each `.md` entry to one `LandmarkFragment` with no protochart, keyed `page:<stem>`, title from the first `#` heading or the file stem, body as one `Content::Paragraph` per blank-line-separated run of text keyed `block:<stem>:<n>`, and `[[name]]` spans as mentions keyed `page:<name>` — the same key a page with that stem would have, so a mention to a page in the fixture resolves to it and a mention to one outside stays a forward reference.
It emits no protochart and parses nothing with the dialect.
Its configuration type has two fields: `title_from: heading | filename`, which exercises the generated schema and validation, and `fail_on_entry: Option<String>`, which makes `entry` panic on the named entry so the trap and `wp_panic` paths have something to exercise.
It is deliberately not useful: the moment it is, it is a product importer and belongs in its own change with its own spec.

It is a crate of its own rather than a file under `crates/transload/tests/` for three reasons: a `cdylib` is a crate-level property, so a module can only come from a separate package; it must depend on `transload` with `sdk` and without `store`, which a test inside `transload` cannot express, since tests compile with the crate's own features; and its wasm build must not drag `transload`'s dev-dependencies.
It is a workspace member rather than an excluded package so it shares the lockfile, `cargo deny`, clippy, treefmt, and the workspace's `[patch.crates-io]` entries, and `transload` excludes the `fixtures` directory from its own package so the nesting is harmless.

Every check that is about the plugin's own dependency graph lives in the fixture crate's tests, not `transload`'s: under `cargo nextest run --workspace` all members' features unify, so a check run from `transload`'s tests would see `store` and every host-side dependency.
The fixture's tests run a nested `cargo build -p plain-markdown --target wasm32-unknown-unknown` into its own target directory (the shape `trybuild` already uses), which is the only build in which the plugin's graph is the plugin's graph, and then hand the artifact to the harness's module backend.
`transload`'s own tests cover what does not depend on the graph: Fragment serialization, the census, the manifest loader, the schema generator, and the macro's exports called on the host target through their `extern "C"` symbols by one test importer — one invocation per test binary, since the names cannot be linked twice.

- **Rejected — a test fixture inside `transload`'s tests:** cannot be a `cdylib`, cannot drop `store`, cannot escape unification.
- **Rejected — a package excluded from the workspace and built by manifest path:** its own lockfile drifts from the workspace's, it loses the specta patch, and the lint and format gates skip it.
- **Rejected — the fixture driving the harness from `transload`'s tests:** a dev-dependency cycle, and two copies of `transload`'s types in one binary.

### D11 — Entries arrive in a defined order: markers first, then sorted paths (2026-10-30)

A host feeds the manifest's marker files first, in the order the manifest lists them, then every remaining entry in byte-wise sorted path order.
A plugin may rely on that order, which is what makes a configuration file that changes how later entries are read (Logseq's `config.end`) a marker rather than a buffering problem.
Determinism is then a property of a run over a defined sequence, which the harness can check; order-independence of per-entry outputs is not required and not claimed.

- **Rejected — "any order the host chooses" (the previous design):** unverifiable, and it would force every plugin with a configuration file to buffer the whole source.

### D12 — The SDK checks protochart consistency at finalize; dangling Mark references are not its concern (2026-10-30)

At `finalize`, after the plugin's own `finalize` returns, the SDK checks across everything the run emitted that every Landmark's protochart key is a protochart the run defined, that every reading key is a feature of that protochart, that every select value is an option (or group) of that feature, that every `Reference.allowed_types` entry is a protochart the run defined, and that a Landmark bound to no protochart carries no readings.
A violation is an `err` envelope naming the Landmark and key.
This is the one place the whole run is visible to the SDK, and these are errors in the plugin, not in the source, so they belong in the plugin's tests, not in a host's import report.

References to Mark keys — mentions, Terrain endpoints, reference readings — are not checked: a key defined by no entry in this run may still be defined by the ledger from an earlier one, and what to do with a key nobody ever defines (the draft's open ◆) is the ingest change's policy.

- **Rejected — checking nothing in the SDK:** every protochart error would surface at import time, in a host, against a user's data.
- **Rejected — checking Mark references too:** false failures for every reference into an earlier run.

### D13 — `id` is reserved; everything else unknown is ignored; byte-stability claims are scoped (2026-10-30)

A Fragment entity ignores fields it does not define, per D4, with one exception: a field named `id` is refused, because the one mistake a direction-tagged grammar exists to make impossible is identity arriving from the wrong side.
Byte-stability is claimed where it can be true: a run over the same configuration and entry sequence produces the same bytes; and deserializing then serializing an output the SDK wrote reproduces it exactly, since the SDK writes canonical form (normalized timestamps, sorted unordered collections, stable field order) and no unknown fields.
It is not claimed that equal values serialize identically, since `IndexMap` equality ignores the order a plugin chose for its features and readings and that order is meaningful.

- **Rejected — refusing all unknown fields:** forbids additive evolution within a major.
- **Rejected — ignoring `id` like any other unknown field:** silently accepting the one field that must never be there.
- **Rejected — "equal values, equal bytes" (the previous design):** false for ordered maps.

## Risks / Trade-offs

- [The nested wasm build in the fixture's tests is slow] → it compiles `domain`, the grammar, `toml`, `schemars`, and their dependencies for a second target, incrementally, in its own target directory; the comparable probe took about thirteen seconds warm, and the task records the fixture's figure, with a lighter profile if the number warrants.
- [A dependency added to the grammar, the data model, or the SDK later needs a host] → the fixture's instantiation under an empty `Linker` fails and names the import; it runs on every hook.
- [A runtime refuses to call exports after a trap, so `wp_panic` is unreachable] → wasmi and wasmtime both leave the instance usable after a trap; if a future runtime does not, the panic buffer's address can additionally be exposed as a global so the host reads memory directly, an additive change within the ABI major.
- [`schemars` output is richer than a simple form renderer expects (`$ref`, `definitions`, `oneOf`-with-`const` enums)] → a constraint on the host change's form renderer, recorded here; the fixture's two-field configuration keeps this change's schema simple.
- [`toml` or `schemars` fails to build for `wasm32-unknown-unknown`] → both are pure Rust; the fixture's nested build is the check, and either failure would be found in the first task that builds it.
- [The Fragment shape turns out wrong for the ingest change] → the §9 rules bound what the ingester consumes; fields can be added within `fragment_format` 0.x without touching this change; the harness's golden files make any reshaping visible.
- [Two macro invocations end up in one test binary] → one test importer, in one integration-test file, in `transload`; the fixture's own tests contain only the fixture.
- [`cargo deny` objects to `wasmi` or a direct `schemars`/`toml` edge] → the licenses are already in the allow-list and two of the three crates are already in the lockfile; anything surfaced is triaged with the file's `ignore`-with-reason discipline.

## Migration Plan

1. `grammar-only-builds` is archived; everything here builds on it as merged.
2. Fragment types and ABI types land behind nothing, then the `sdk` feature with the trait, manifest loader, schema generator, panic hook, and macro, then the `harness` feature with both backends, then the fixture crate with its goldens and graph checks — each hook-green.
3. Rollback: `sdk` and `harness` default off; removing the fixture crate and the two features leaves the grammar modules and the Fragment types as data with tests.

## Open Questions

Deferrable here; each belongs to the change named.

- Where unmapped readings go when a protochart is mapped onto a Chart lacking those features, the page fallback included — namespaced `extra` is the obvious home.
  (Fragment ingestion.)
- How the ledger keys protochart, feature, and option mappings — the archived D4 keys rows on `(importer, source_key)`; mapping rows are kind-qualified.
  (Fragment ingestion.)
- Whether reading-kind terrain edges are ever materialized from reference readings.
  (Data model.)
- Whether the harness should also assert a maximum module size for the fixture, as an early guard against a dependency quietly bloating plugins; the probe measured 268 092 bytes with `opt-level = "s"` and LTO.
  (This change, once the fixture's own number is measured.)
