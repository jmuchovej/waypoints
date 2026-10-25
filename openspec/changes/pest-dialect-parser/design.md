## Context

See proposal.md — Why. What shapes the approach:

- The parser is already PEG-shaped. `inline::Parser::atom` tries mention,
  bracketed run, code, math, braced script, then the paired shortcuts, in a
  fixed order — ordered choice. Runs end at a closer found by scanning —
  negative lookahead. Variable-length code fences are matched by remembering
  the opener — a PEG stack.
- Three layers exist today: inline (`inline.rs`), block (`block.rs`: one
  block's own lines — attribute list, fence, `::` component, GFM rows), and
  document (`transload::ingest::parse_region`: indentation, `wp:mark` comment
  pairing, which lines belong to which block). Only the first two are in scope.
- Both directions share one token table (`TOKEN_ORDER`, `PAIRED`) so a
  spelling cannot drift; `pest` cannot read Rust constants, so the port
  introduces a second statement of the tokens.
- Refusals from `transload` ingest carry `Location { file, line }` plus a
  `GrammarError { detail, at }` from the dialect; `at` is a byte offset into
  the block's source. Messages are prose ("a bracketed run needs a `(url)` or
  `{tokens}` suffix; a literal bracket is escaped") and tests assert on them.
- `pest`/`pest_derive` build on stable Rust; the workspace pins 1.98.1.
- House rules: hooks green per commit; treefmt owns formatting (it has no
  `.pest` formatter, so grammar files are formatted by hand and excluded).

## Goals / Non-Goals

**Goals:**

- `inline.pest` and `block.pest` as the executable grammar; `inline::parse_with`
  and `block::parse_with` implemented as walkers over the generated pair tree.
- Byte-for-byte identical acceptance on everything the emitter writes, proven
  by the existing tests, the conformance suite, and a differential run against
  the old parser during migration.
- Refusal messages at least as specific as today's, located by line and column.

**Non-Goals:**

- A grammar-derived emitter (D4).
- The document layer (D2).
- Any dialect change, however small, discovered during the port. A discrepancy
  between the old parser and the grammar is a bug in one of them; it is fixed
  so they agree, and the dialect spec is not reopened.

## Decisions

### D1 — Two grammar files, one per layer (2026-10-23)

`inline.pest` covers a run of inline text. `block.pest` covers one block's own
lines and reuses the inline grammar for its text positions by handing each
text span to `inline::parse_with` rather than importing the inline rules
(pest has no `include`; a single combined file would be long and would blur
the layer boundary that `block::Rendered` already draws).

_Rejected:_ one combined grammar — the block grammar would then have to carry
the inline rules verbatim, and the document walker's line slicing already
delivers block source one block at a time.

### D2 — The document layer stays a line walker (2026-10-23)

`ingest::parse_region` keeps its hand-written form. Two reasons: refusals owe
a file line number, which a line walker has for free and a byte-offset PEG
over a whole file would have to recompute; and indentation sensitivity in
pest means `PEEK_ALL`/`PUSH` over leading whitespace with a stack reset at
every container, which is the one shape where a PEG is harder to read than
the loop it replaces. The walker already hands each block its own de-indented
source, which is exactly the input `block.pest` wants.

_Rejected:_ a `document.pest` over the whole body — possible, but the review
cost of an indentation PEG exceeds the benefit while the walker is ~150 lines.
Revisit as a follow-up once the two grammars have settled.

### D3 — Error mapping through renamed rules (2026-10-23)

pest's `Error::renamed_rules` maps each rule to a display name; the grammar
names rules so the map reads as the dialect's vocabulary (`bracketed_run`,
`attribute_list`, `mention_target`). `GrammarError` keeps `detail: String` and
`at: usize` (byte offset) and gains `line`/`column` from pest's position;
`Display` prints line and column. Where the old parser emitted a hand-written
sentence for a specific failure (the bracketed-run suffix rule), the walker
keeps producing that sentence: the grammar admits a bare `[text]` and the
walker refuses it with the existing message, since a PEG cannot say "this is
wrong because of what would have made it right".

_Rejected:_ exposing pest's `Error` directly — its messages name rules and
its type would leak a dependency into `domain`'s public surface.

### D4 — The emitter stays hand-written; parity is tested, not derived (2026-10-23)

No maintained Rust library derives a pretty-printer from a PEG, and the
invertible-syntax approach (one description yielding both parser and printer)
has no production Rust implementation. The dialect's canonical form is also
not a grammar property — "one mark with a shortcut takes the shortcut, else
the span" is a policy over equivalent spellings — so a derived printer would
still need that policy written by hand.

The emitter therefore keeps `TOKEN_ORDER`, `PAIRED` and the escape tables.
Drift between those and the grammar is caught by a test that, for every
`TextMarkKind` and every data-bearing token value the emitter can produce,
renders a one-segment `RichText` and parses it back; and the existing
property that every `Content` variant survives its block form. These are the
"emitter writes only what the grammar reads" scenarios.

_Rejected:_ generating `TOKEN_ORDER` from the `.pest` file (or the reverse) in
`build.rs` — one more generator for a ten-row table; the parity test is
cheaper and says the same thing.

### D5 — Differential migration, then delete (2026-10-23)

The old parser moves to `syntax::legacy` (private) for the duration of the
port. A test parses the conformance generator's inline and block output with
both and asserts equality, including on refusal-versus-accept. When the
grammar matches on 10 000 generated stores and the committed Bundle, the
legacy module is deleted in the same change. This is what lets the port be
reviewed as "same language, new parser" rather than trusted.

### D6 — Grammar files are normative; `dialect.md` defers (2026-10-23)

`dialect.md` gains a short "Executable grammar" section naming the two files
and stating that they win on disagreement. Prose that restates a production
is cut to the rationale (why the scripts are braced, why `~` is escaped only
when doubled); prose that states policy the grammar cannot carry (canonical
form, normalization, mention naming) stays normative in the document.

## Risks / Trade-offs

- [pest's PEG is greedy and never backtracks across `~` once a choice commits]
  → the grammar is written to mirror the old parser's commit points (a `[[`
  commits to a mention; a `[` commits to a bracketed run), and the
  differential test catches any case where the old parser's lookahead did
  something the grammar does not.
- [Escapes inside verbatim interiors (`` ` ``, `$`) differ from escapes in
  prose] → separate atomic rules per interior; the existing "code and math
  interiors are literal" test pins it.
- [A second statement of the token table] → D4's parity test; a token added
  to one place and not the other fails the test rather than the round trip.
- [Compile-time cost of `pest_derive`] → it is a proc macro over two small
  files; measured once in task 1.3 and recorded. `syntax` is a feature, so
  `domain` consumers that do not parse pay nothing.
- [Message regressions] → every existing refusal assertion in `inline.rs`,
  `block.rs`, and `transload/tests/ingest.rs` stays as written; D3 keeps the
  hand-written sentences where the grammar alone cannot produce them.
- [treefmt has no `.pest` formatter] → the two files are excluded in
  `treefmt.toml` with a one-line reason, like the `getting-started` Bundle.

## Migration Plan

Each task is a commit with hooks green. Order: grammar files and dependency →
inline walker behind the differential test → block walker → error mapping →
delete `legacy` → documentation. Rollback at any point before the deletion is
`jj abandon` of the walker commits; the old parser is still the live one until
the legacy module is removed.

## Open Questions

- Whether `pest`'s `Error` line/column (1-based, char-based) or a byte-offset
  pair better serves the `ingest` refusal report that already carries a file
  line. Deferrable: `GrammarError` carries both, and the report format can
  pick later without touching the grammar.
