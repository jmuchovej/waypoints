# importer-abi — Draft 0.1 (historical record)

Pulled from Notion on 2026-10-29 (page `importer-abi`, last edited 2026-08-17, child of the `Waypoints` page).
This is the drafting record the `transload-sdk` change reverse-engineers; where it disagrees with the change's spec deltas and `design.md`, those win.
Markdown conversion is faithful to the page; nothing was added or resolved here.
`◆` marks a decision the draft left open.

> **Status:** Draft 0.1 · 2026-08-17 · normative language per RFC 2119 · depends on projection-spec ≥ 0.3
>
> **What this is:** the frozen WASM surface importer plugins build against.
> A plugin is a pure function from foreign bytes to Fragments (projection-spec §2); everything stateful, dangerous, or identity-bearing is the core's.
>
> **Decision markers:** ◆ = John rules before 0.2.
> Collected in §9.

## 1. Design invariants

1. **Pure function.**
   A plugin transforms source bytes into Fragments.
   No filesystem, no network, no clock, no randomness — the module imports _zero_ host functions, which makes the sandbox story a property you can verify with `wasm-objdump`, not a promise.
2. **Determinism (normative).**
   Same config + same entries in the same order ⇒ byte-identical output.
   Minting is exclusively the core's, so plugins have nothing legitimate to be random about.
   Enables caching on `(plugin-hash, config, entry-hashes)` and golden tests.
3. **Grammar split enforced at the boundary.**
   Fragments carry `src` and never `id` (projection-spec §2) — with one carve-out: `builtin.*` ChartIDs are stable public constants, not minted, so fragments MAY bind to them directly.
4. **Small and frozen.**
   Seven exports, one buffer convention, JSON payloads with unknown-fields-preserved.
   Additive evolution only; a breaking change is a new export name, not a changed one.
5. **Streaming, single pass.**
   The host walks the source and feeds entries one at a time; peak plugin memory tracks the largest single file, not the graph.
   Cross-file references need no global view because they're expressed by source key and resolved by the ingester (§5) — multi-pass designs are rejected by construction.

## 2. Lifecycle

`wp_abi` → `wp_manifest` → `wp_init(config)` → `wp_entry(state, name, bytes)` × N → `wp_finalize(state)`

The host resolves the source (directory, zip, single payload — the share-sheet case is `init → one entry → finalize`, which is why mobile costs the ABI nothing), validates the manifest, renders the options form, then streams entries.
The WASM instance lives for the whole run; `state` is a guest-internal handle.

## 3. Exports & buffer convention

All data crosses as guest-memory buffers.
`u64` returns pack `(ptr << 32) | len`; the host copies out, then calls `wp_free`.
Every fallible call returns a `Result` payload: `{"ok": …}` or `{"err": {"msg": …, "at": …?}}`.
A trap aborts the run with a report.

| Export        | Signature (wasm32)                                         | Returns                                 |
| ------------- | ---------------------------------------------------------- | --------------------------------------- |
| `wp_abi`      | `() -> u32`                                                | ABI version constant; host checks first |
| `wp_alloc`    | `(len: u32) -> u32`                                        | guest pointer; host writes inputs here  |
| `wp_free`     | `(ptr: u32, len: u32)`                                     | —                                       |
| `wp_manifest` | `() -> u64`                                                | Manifest JSON (§4)                      |
| `wp_init`     | `(cfg_ptr, cfg_len) -> u64`                                | `Result<{"state": u32}>`                |
| `wp_entry`    | `(state, name_ptr, name_len, bytes_ptr, bytes_len) -> u64` | `Result<EntryOutput>`                   |
| `wp_finalize` | `(state) -> u64`                                           | `Result<FinalizeOutput>`                |

## 4. Payloads

**Manifest** — static identity + what the host UI needs:

```json
{
  "abi": 1,
  "id": "logseq",
  "version": "0.1.0",
  "namespace": "so.waypoints.importer.logseq",
  "source": {
    "kinds": ["directory"],
    "markers": ["logseq/config.edn"],
    "extensions": [".md"]
  },
  "options": {}
}
```

`options` is a Draft-07 JSON Schema; the host renders the config form from it and validates user input against it before `wp_init` — the same schema machinery the Bundle already relies on, pointed at plugin configuration.

**EntryOutput / FinalizeOutput:**

```json
{
  "fragments": [],
  "asset": { "from_entry": true, "ext": "png", "src": { "key": "…" } },
  "diagnostics": [{ "level": "warn", "msg": "…", "at": "pages/foo.md:12" }]
}
```

When an entry _is_ an asset, the plugin marks it (`from_entry: true`) and the host content-addresses the bytes it already holds — bytes never round-trip through the guest.
Plugin-_generated_ assets (format conversions) MAY inline base64 in 0.1 — ◆.
`FinalizeOutput` adds a `summary` (counts) and carries any source-global fragments (favorites, graph-level config) that only exist once the stream ends.
Diagnostics ride in-band — no host logging import — and become the import report.

## 5. Fragments reference by source key

The load-bearing simplification: **references between imported entities are expressed by source key, and resolution is the ingester's job.**
A Logseq block-ref to a block in a file the plugin hasn't seen yet is just `{"ref": {"src": "<logseq-uuid>"}}` — no global view required, which is what makes §1.5's single pass honest.
Fragment grammar mirrors Bundle grammar with `src` in place of `id`:

- Identity comments: `<!-- wp:mark {"src": {"key": "…"}} -->`
- Reading refs: `{"ref": {"src": "…"}, "label": "…"}`
- Terrain lines: `{"src": "key-a", "dst": "key-b", "kind": "…", "order": 0, "extra": {…}}`
- Chart bindings: `"chart_src": "db:tasks"` for importer-defined Charts, or `"chart": "builtin.page"` for built-ins (the §1.3 carve-out)

**◆ Cross-doc obligation:** projection-spec needs a short Fragment-grammar subsection making the `src` forms above normative — currently implied by §2's definition, not specified.

Dangling srcrefs (a key never seen in this run): ◆ — flag-for-review (proposed, consistent with the tombstone rule) vs. placeholder Marks.

## 6. Runtime & limits

Desktop: `wasmtime` with fuel metering and a memory cap (defaults ◆ — implementation numbers).
Mobile-later: `wasmi` (pure-Rust interpreter, no JIT, App-Store-safe) or the webview's blessed engine — invisible to plugins either way, because the ABI is dumb buffers.
Entry-at-a-time feeding bounds peak memory at the largest single file.
Performance claims wait for numbers, per the earlier ruling.

## 7. SDK & conformance

The Rust SDK reduces the ABI to one trait plus a macro:

```rs
pub trait Importer: Sized {
    fn manifest() -> Manifest;
    fn init(cfg: Config) -> Result<Self, Diag>;
    fn entry(&mut self, name: &str, bytes: &[u8]) -> Result<EntryOutput, Diag>;
    fn finalize(self) -> Result<FinalizeOutput, Diag>;
}
wp_importer!(LogseqImporter);
```

Conformance = golden fixtures: `fixtures/` in, expected Fragments out, byte-compared (determinism makes this exact); where an in-process ingester is available, the harness also asserts "ingests without report."
Golden fixtures are the same artifacts that generate the migration matrix's fidelity column (projection-spec §10) — the what-survives table cites this suite.
The Logseq importer ships in-tree as the first plugin, dogfooding every line of this page.

## 8. Versioning & the freeze

`wp_abi` returns the frozen major.
Payload evolution is additive: new OPTIONAL fields, unknown fields preserved (mirroring projection-spec §4.3).
Anything else is `wp_abi = 2` alongside, not instead.
The contract itself is tier-agnostic — a TypeScript-tier importer would implement the same trait against the same payloads; WASM is the polyglot door, not the contract.

## 9. ◆ Decision ledger for Draft 0.2

- [ ] ◆ Payload encoding: JSON (drafted, debuggable) vs. msgpack (smaller; makes inline asset bytes sane) — §3/§4
- [ ] ◆ Plugin-generated assets: inline base64 in 0.1, revisit with encoding — §4
- [ ] ◆ Dangling srcref policy: flag-for-review (proposed) vs. placeholder Marks — §5
- [ ] ◆ Fragment-grammar subsection lands in projection-spec (cross-doc) — §5
- [ ] ◆ Fuel + memory defaults — §6

- Resolved by design: zero host imports (§1.1) · single streaming pass via srcrefs (§1.5) · determinism as a MUST (§1.2) · builtin.\* carve-out (§1.3)
