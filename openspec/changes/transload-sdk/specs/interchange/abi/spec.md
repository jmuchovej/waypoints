## Purpose

The contract between a host and an importer plugin: a WebAssembly module that turns foreign bytes into Fragments through a fixed set of exports, imports nothing, and is deterministic by construction — plus what the SDK and conformance harness guarantee about a plugin built with them.

## ADDED Requirements

### Requirement: A plugin module has a fixed shape and imports nothing

An importer plugin SHALL be a WebAssembly module for the `wasm32-unknown-unknown` target whose function exports are exactly `wp_abi`, `wp_alloc`, `wp_free`, `wp_manifest`, `wp_init`, `wp_entry`, `wp_finalize`, and `wp_panic`, whose linear memory is exported, and whose import section is empty.
A module that imports anything — a function, a memory, a table, or a global, from any module name — is not a plugin.

#### Scenario: The fixture plugin has the required shape

- **WHEN** the fixture plugin is built for `wasm32-unknown-unknown` and its module is inspected
- **THEN** its function exports are exactly the eight ABI names
- **AND** it exports its memory
- **AND** its import section is empty

#### Scenario: A module with an import cannot be instantiated

- **WHEN** a module that declares any import is given to a host offering an empty import table
- **THEN** instantiation fails, naming the import
- **AND** the conformance harness reports the module as not a plugin

### Requirement: The export signatures are frozen

The eight exports SHALL have exactly these signatures, in wasm32 terms:

| Export        | Signature                                                                           | Returns                                                 |
| ------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `wp_abi`      | `() -> u32`                                                                         | the ABI major                                           |
| `wp_alloc`    | `(len: u32) -> u32`                                                                 | a guest pointer to at least `len` bytes                 |
| `wp_free`     | `(ptr: u32, len: u32)`                                                              | —                                                       |
| `wp_manifest` | `() -> u64`                                                                         | the manifest JSON                                       |
| `wp_init`     | `(cfg_ptr: u32, cfg_len: u32) -> u64`                                               | `Result` of `{"state": u32}`                            |
| `wp_entry`    | `(state: u32, name_ptr: u32, name_len: u32, bytes_ptr: u32, bytes_len: u32) -> u64` | `Result` of an entry output                             |
| `wp_finalize` | `(state: u32) -> u64`                                                               | `Result` of a finalize output                           |
| `wp_panic`    | `() -> u64`                                                                         | the last panic's message and location, or `0` when none |

Every nonzero `u64` return SHALL pack a guest buffer as `(ptr << 32) | len`.

#### Scenario: Signatures are checked

- **WHEN** the fixture plugin's module is inspected
- **THEN** each of the eight exports has exactly the listed parameter and result types

### Requirement: All data crosses as guest-owned buffers

Every input a host passes SHALL be written into guest memory obtained from `wp_alloc`, and every output SHALL be returned as a guest buffer packed into the `u64`.
A host copies an output out and releases it with `wp_free`; the guest never reads memory it did not allocate.
`wp_free(ptr, len)` releases exactly a buffer previously returned by `wp_alloc` or as an output.

#### Scenario: A manifest round-trips through guest memory

- **WHEN** a host calls `wp_manifest` and unpacks the returned `u64`
- **THEN** the bytes at that pointer and length are the manifest JSON
- **AND** `wp_free` of that pointer and length succeeds

#### Scenario: Native and module runs agree byte for byte

- **WHEN** the fixture plugin is driven through the trait natively and through its module's exports
- **THEN** every output is byte-identical between the two

### Requirement: Fallible calls return a result envelope; a trap aborts the run and keeps its message

`wp_init`, `wp_entry`, and `wp_finalize` SHALL return a JSON object that is either `{"ok": <payload>}` or `{"err": <diagnostic>}`, where the diagnostic has the shape of the in-band diagnostics below.
`wp_manifest` returns no envelope; a manifest the SDK cannot serve SHALL trap.
A trap raised by any export SHALL abort the whole run; a host discards every output of that run and reports the trap.
A panic SHALL store its message and location before trapping, and `wp_panic` SHALL return them afterwards; a host that catches a trap SHALL call `wp_panic` and include its text in the trap report.
A plugin SHALL NOT signal failure by any other means.

#### Scenario: Rejected configuration is an error envelope

- **WHEN** `wp_init` is called with configuration the plugin cannot accept
- **THEN** it returns `{"err": …}` and no state handle

#### Scenario: A panic is a trap with its message

- **WHEN** the fixture plugin is configured to panic on a named entry and that entry is fed under the harness's module backend
- **THEN** the call traps rather than returning
- **AND** the run is aborted with no output retained
- **AND** `wp_panic` returns the panic's message and location

#### Scenario: No panic, no message

- **WHEN** `wp_panic` is called on an instance that has not panicked
- **THEN** it returns `0`

### Requirement: The lifecycle is one instance, one order

A run SHALL proceed `wp_abi` → `wp_manifest` → `wp_init` → `wp_entry` × N → `wp_finalize` against a single module instance.
`wp_init` SHALL return a state handle that every later call passes back; `wp_finalize` consumes it.
Entries SHALL be fed one at a time, each as a name and its complete bytes, in a defined order: the manifest's marker files first, in the order the manifest lists them, then every remaining entry in byte-wise sorted path order.
A plugin MAY rely on that order and SHALL NOT require any other.

#### Scenario: Markers lead

- **WHEN** a source contains a marker file named in the manifest and other entries
- **THEN** the marker is fed before any other entry
- **AND** the remaining entries arrive in sorted path order

#### Scenario: Finalize carries what only the whole stream knows

- **WHEN** the last entry has been fed and `wp_finalize` is called
- **THEN** its output MAY carry Fragments that depend on every entry
- **AND** the state handle is invalid afterwards

### Requirement: The manifest has authored fields and stamped fields

`wp_manifest` SHALL return a JSON object carrying the author's fields — `id` (a stable plugin identifier), `name` (a display name), `icon`, `namespace` (a reverse-DNS key that scopes every source key and `extra` entry the plugin writes), and `source` (the kinds of source it reads: directory, archive, or file; marker paths; extensions) — and the SDK's fields — `abi` (the ABI major), `version` (the plugin crate's version), `fragment_format`, `domain_version`, and `options`.
The author's fields SHALL be written as a TOML document beside the plugin's crate; a document missing a required field or carrying a field the manifest does not define SHALL be rejected.
The SDK's fields SHALL be stamped, never authored.

#### Scenario: A complete manifest is served

- **WHEN** the fixture plugin's `wp_manifest` is unpacked
- **THEN** it carries `id`, `name`, `icon`, `namespace`, and `source` as written in its TOML
- **AND** `version` equals the fixture crate's version, `abi` is `1`, and `fragment_format` and `domain_version` are the SDK's constants

#### Scenario: A misspelled field is rejected

- **WHEN** a plugin's TOML manifest carries a field name the manifest does not define
- **THEN** calling the plugin's `manifest()` natively fails naming the field
- **AND** the harness reports it before any entry is fed

### Requirement: The options schema is generated from the configuration type and validated before init

`options` SHALL be a JSON Schema Draft-07 document generated from the plugin's configuration type, declaring Draft-07 in its `$schema`.
Configuration passed to `wp_init` SHALL validate against it; a host validates before calling, and a plugin MAY assume `wp_init` never receives configuration the schema rejects.

#### Scenario: The schema is Draft-07 and valid

- **WHEN** the fixture plugin's `options` is inspected
- **THEN** its `$schema` names Draft-07
- **AND** it validates against the Draft-07 meta-schema
- **AND** it describes exactly the fields of the fixture's configuration type

#### Scenario: Invalid configuration never reaches the plugin

- **WHEN** the conformance harness is given configuration the options schema rejects
- **THEN** it reports the schema violation and does not call `wp_init`

### Requirement: A plugin is deterministic by construction

For a given configuration and a given entry sequence, a plugin's outputs SHALL be byte-identical across runs.
A plugin SHALL have no way to observe a clock, randomness, an environment, or any host state: its module imports nothing, and the data model it links provides no identifier minting and no clock on its target.
A host SHALL instantiate a plugin with an empty import table and SHALL provide nothing else.

#### Scenario: Two runs, one output

- **WHEN** the fixture plugin is run twice over the same fixture directory with the same configuration
- **THEN** every `wp_entry` output and the `wp_finalize` output are byte-identical between the runs

#### Scenario: Nothing to serve

- **WHEN** the fixture plugin is instantiated under the harness's module backend
- **THEN** the host provides no functions, memories, tables, or globals
- **AND** instantiation succeeds

### Requirement: Diagnostics ride in-band

Every `wp_entry` and `wp_finalize` output SHALL carry a `diagnostics` list.
A diagnostic SHALL have a `level` (`info`, `warn`, or `error`), a `msg`, and an optional `at` naming the entry and, where the entry has lines, a line.
A plugin SHALL NOT report outside its outputs; a condition worth reporting that does not stop the run is a diagnostic, one that does is an `err` envelope, and a panic is a trap whose message `wp_panic` keeps.

#### Scenario: A skipped construct is reported where it was found

- **WHEN** the fixture plugin meets a construct it does not carry in an entry
- **THEN** that entry's output includes a `warn` diagnostic naming the entry and line

### Requirement: Finalize checks the run's protocharts

At finalize, the SDK SHALL check across the whole run that every Landmark's protochart key names a protochart the run defined, that every reading key is a feature of that protochart, that every select value is an option or group of that feature, that every reference feature's allowed protocharts were defined, and that a Landmark bound to no protochart carries no readings; a violation SHALL be an `err` envelope naming the Landmark and the key.
References to Mark keys SHALL NOT be checked by the SDK.

#### Scenario: A reading on an undefined feature fails finalize

- **WHEN** a plugin emits a Landmark whose reading names a feature key its protochart does not define
- **THEN** `wp_finalize` returns `{"err": …}` naming the Landmark and the key

#### Scenario: A forward reference to a Mark passes

- **WHEN** a plugin emits a mention to a Mark key no entry in the run defines
- **THEN** finalize succeeds

### Requirement: The ABI is frozen and payloads evolve additively

`wp_abi` SHALL return the ABI major, `1`.
Within a major, the exports and their signatures SHALL NOT change, and payload evolution SHALL be additive: a new field is optional, and a reader SHALL ignore a field it does not know.
A change that cannot be additive SHALL be a new major, offered alongside the old, never a redefinition.
The finalize output SHALL carry a census of the content, reading, feature-kind, and terrain-kind variants the run emitted, computed from the output, so a reader can tell whether a newer grammar's additions are present.

#### Scenario: An unknown field is not an error

- **WHEN** an output carries a field the reader's envelope version does not define
- **THEN** the reader accepts the output and ignores the field

#### Scenario: The ABI major is checked first

- **WHEN** a host opens a module whose `wp_abi` returns a major it does not support
- **THEN** it calls nothing else

#### Scenario: The census is computed

- **WHEN** a run emits paragraphs and headings and nothing else
- **THEN** its finalize output's census lists exactly those content variants

### Requirement: The SDK makes a plugin a trait, a macro, and a TOML file

The SDK SHALL expose the plugin contract as one trait — `manifest`, `init` (configuration in, plugin or diagnostic out), `entry` (name and bytes in, output or diagnostic out), `finalize` (consuming the plugin, output or diagnostic out) — with a configuration type that is deserializable and schema-describable, and one macro that, given an implementation and the path of its TOML manifest, produces the eight exports and installs the panic hook.
A plugin written against them SHALL satisfy every plugin-side requirement in this capability with no further code.
The trait is plain Rust, so a plugin's own tests SHALL run natively without a WebAssembly runtime.

#### Scenario: The macro is the whole ABI

- **WHEN** a type implements the trait and the macro is applied to it with its manifest
- **THEN** building the crate as a `cdylib` for `wasm32-unknown-unknown` yields a module satisfying the shape, signature, buffer, envelope, lifecycle, manifest, and schema requirements above

#### Scenario: A plugin tests natively

- **WHEN** a plugin's unit tests call its trait methods directly
- **THEN** they run on the host target with no module built

### Requirement: Conformance is golden fixtures, determinism, and module parity

The SDK SHALL provide a harness that runs a plugin over a fixture directory in the ABI's entry order, validates the configuration, serializes every output, and compares each against a committed expected file byte for byte — natively through the trait, and again from the plugin's compiled module instantiated with an empty import table — and requires the two runs to agree.
A plugin is conformant when every expected file matches, two runs produce identical output, the module run agrees with the native run, and the module's shape satisfies the first requirement.

#### Scenario: A changed output is a failed golden

- **WHEN** a plugin's output for a fixture entry differs by one byte from the committed expectation
- **THEN** the harness reports that entry and the difference

#### Scenario: The fixture plugin is conformant

- **WHEN** the harness runs the fixture plugin over its fixtures
- **THEN** every golden file matches, both runs agree, and the module has the required shape

## Decisions

- **2026-10-29 — Typed JSON Fragments, not Bundle-grammar text.**
  The draft's text Fragments mirrored a grammar that has since changed under it, would need four `src` forms the ingester refuses on purpose, and type-check nowhere.
  Typed payloads are validated by deserialization at the boundary.
  Rejected: text Fragments; untyped JSON values.
  See design D1.
- **2026-10-29 — JSON payloads.**
  Golden fixtures diff as text; the one case for a binary encoding (inline asset bytes) is out of scope.
  Rejected: msgpack.
  See design D4.
- **2026-10-29 — Three version numbers.**
  `wp_abi` for the surface, `fragment_format` for the envelope, `domain_version` for the grammar; the census is computed from output at finalize, never authored.
  Rejected: deriving envelope versioning from the ABI major; an authored census.
  See design D4.
- **2026-11-02 — `wasm32-unknown-unknown`, an empty import section, and `wp_panic`.**
  The sandbox is a property of the build the archived `grammar-only-builds` change produced; nothing in a plugin can reach a host, so nothing is served, fixed, or counted.
  Standard error does not exist on the target, so a panic hook keeps the message and an eighth export returns it after the trap.
  Rejected: `wasm32-wasip1` with a deterministic host that serves a fixed clock and seeded randomness and counts calls (the previous design, made unnecessary by the gated data model); a WASI allowlist (nothing to allow); losing the panic message; a panic-buffer global in place of the export.
  See design D6, D7.
- **2026-10-30 — Markers first, then sorted paths.**
  A defined order makes determinism checkable and lets a configuration file govern later entries without buffering.
  Rejected: host-chosen order with order-independent outputs (unverifiable, and it forces buffering).
  See design D11.
- **2026-10-30 — The manifest is TOML the author writes; the schema is generated from the configuration type.**
  A static description reads and diffs as a file; a hand-written schema beside a configuration type drifts from it.
  Rejected: a manifest as a Rust value; JSON Schema spelled in TOML; a TOML-to-schema language; a hand-authored schema; an authored `version`.
  See design D8.
- **2026-10-30 — Finalize checks protocharts, not Mark references.**
  Protochart errors are the plugin's and belong in its tests; a Mark key undefined in this run may be defined by an earlier one.
  Rejected: checking nothing; checking Mark references.
  See design D12.
- **2026-10-29 — No asset marker.**
  The draft's `asset` field waits on an asset model in the data model; omitted rather than reserved.
  Rejected: reserving the field with no semantics.
- **2026-11-02 — Conformance runs natively and as a module under wasmi with an empty linker.**
  Native proves the plugin; the module proves the exports, the buffer convention, and the empty import section; parity proves they are the same plugin.
  Rejected: inspecting the module without running it; a runtime-abstraction trait; driving tests through the application's future runtime; a host that provides shims.
  See design D9.
