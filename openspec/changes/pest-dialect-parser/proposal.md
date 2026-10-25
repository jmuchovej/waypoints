## Why

The dialect's normative statement is prose (`crates/domain/docs/dialect.md`),
and the only thing that checks the prose is a hand-written recursive-descent
parser spread across three places: the inline lexer in `inline.rs`, the
block-line scanners in `block.rs` (`split_attrs`, `split_tokens`,
`split_cells`, the fence and component readers), and the document walker in
`transload`'s `ingest.rs` (`parse_region`/`parse_node`: indentation, `wp:mark`
pairing, fenced regions, tables). Every rule the generator-driven conformance
suite forced in `transload-pipeline` — edge escapes, adaptive fences, a trailing
brace that reads as attributes, a cell that looks like a delimiter row — landed
as a fix in one of those three places and a sentence in the prose, with nothing
tying the two together. A Bundle is meant to be read for decades; its grammar
should be a single executable artifact that the parser is derived from and the
reference document defers to, so a reviewer can read the grammar instead of
reverse-engineering it from `match` arms.

## What Changes

- The inline and block grammars become PEG grammar files shipped in `domain`
  (`crates/domain/src/syntax/grammar/inline.pest`, `block.pest`); the parsers
  are generated from them with `pest`/`pest_derive`. The public surface of
  `domain::syntax` (`inline::parse`/`parse_with`, `block::parse`/`parse_with`,
  `Links`, `GrammarError`) is unchanged.
- The grammar files become the normative statement of the dialect's inline and
  block syntax. `dialect.md` stays the reference document — it explains the
  design and names the rules — but where the two disagree, the grammar wins,
  and the document says so.
- A grammar refusal names the construct that was expected as well as the
  position, mapped from grammar rule names to the dialect's own vocabulary
  (never raw rule identifiers). Positions become line and column within the
  block's source; `GrammarError.at` keeps its byte offset for callers that
  already read it.
- Parsing is total: any input is either parsed or refused, never a panic or a
  non-terminating loop. PEG grammars give termination structurally; the
  generated parser is wrapped so that no `unwrap` on a pair reaches a caller.
- The emitter stays hand-written. No Rust library derives a printer from a
  PEG, and the dialect's canonical-form rules (one spelling per mark set,
  coalescing, token order) are decisions, not grammar. The emitter keeps its
  shared token tables, and a test proves every token the emitter can write is
  a token the grammar reads (see design D4).
- The document layer in `transload::ingest` (indented tree, `wp:mark` comment
  pairing, fenced and tabular regions) stays line-oriented and hand-written in
  this change. It is the part that owes refusals a file line number, and
  indentation-sensitive PEGs are the one place `pest` is awkward (see design
  D2, Non-goals).

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `transload`: the requirement that stripped projection files "still parse
  under the dialect grammar" gains the grammar itself as a shipped,
  machine-checkable artifact; refusals of malformed inline or block source name
  the expected construct and a line and column; parsing is total.

## Non-goals

- Changing the dialect. Every byte the current emitter writes parses to the
  same `RichText`/`Content` after this change; the conformance suite and the
  committed `getting-started` Bundle are the gate.
- A grammar-derived emitter. Recorded as rejected in design D4; revisit only if
  a maintained Rust library appears.
- Porting the document layer (`ingest::parse_region`) to a PEG. Noted as a
  possible follow-up once the inline and block grammars have settled.
- Parsing the legacy `importer` crate's grammar. It is dormant and unrelated.
- A public API stability promise for `domain::syntax`; the dialect is
  versioned by the Bundle manifest, not by the Rust surface.

## Impact

- `crates/domain`: two `.pest` files; `inline.rs` loses `Parser` and becomes a
  pair-tree walker plus the unchanged emitter; `block.rs` loses its scanners.
  New dependencies behind the `syntax` feature: `pest` and `pest_derive` (a
  proc macro; MIT/Apache-2.0, already allowed by `cargo deny`).
- `crates/transload`: no source changes expected beyond error-message
  assertions in tests; `ingest.rs` keeps the line walker.
- `crates/domain/docs/dialect.md`: a section pointing at the grammar files and
  stating precedence; prose that restates grammar productions is trimmed to
  the rationale.
- Tests: existing inline/block unit tests and the `transload` conformance
  suite are the acceptance gate; a differential test against the retained
  hand-written parser runs during the migration and is deleted with it.
- Compile time: one more proc-macro crate in the `domain` dependency graph for
  `syntax` consumers (`transload`, `store` under `mock`). No app-facing change.
