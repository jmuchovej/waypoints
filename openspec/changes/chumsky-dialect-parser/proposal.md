## Why

The dialect has one normative document (`crates/domain/docs/dialect.md`) and
four hand-written readers of it, none of which can say what it expected at a
given position. Each is a cursor over bytes with its own bookkeeping:

- `inline.rs` is a `Parser { src, at, segments, pending }` whose every method
  moves `at` by hand — `self.at += first.len_utf8() + escaped.len_utf8()`
  (`literal_char`), `self.at += open.len() - 1; self.at += 1;` (`braced`,
  with a comment explaining why) — and rewinds it to the opener in seven
  places before returning an error. Backslash escapes are decoded in three of
  its methods (`literal_char`, `mention`, `delimited`), and `mention` is a
  character-level state machine over `in_label` and `block`.
- `block.rs` adds three more escape decoders that cannot share code with the
  inline ones because each scans a different way: `split_attrs` walks a line
  _backwards_ counting odd runs of backslashes and toggling `quoted` to find
  the opening brace of a trailing attribute list; `split_tokens` walks the
  interior forwards with `quoted` and `escaped` flags; `split_cells` undoes
  `\|` and infers an escaped trailing space from an odd backslash run after a
  `trim`. The trailing-backslash cell bug the conformance suite found at seed
  859 lived in the last of these.
- `ingest.rs` walks the body as `Vec<(usize, String)>`: each nesting level
  strips its prefix and _copies_ its subtree into a fresh vector
  (`children.push((*number, stripped.to_string()))`), `join` rebuilds a
  block's source for `block::parse_with`, and the refusal that comes back
  carries `(at byte N)` of a string that exists nowhere in the Bundle.
  `matching_closer` is another scanner — depth counter plus fence state —
  that exists only to find where a `::` component ends.
- `frontmatter.rs` parses with `saphyr`, a full YAML 1.2 reader, and then
  spends 270 lines re-deriving a strict reading from a permissive tree:
  `as_text` accepts six scalar shapes because YAML may have re-typed a value
  the emitter wrote as text, and every refusal is `Grammar { detail }` with no
  line, because `saphyr`'s tree has none to give.

Errors are a `GrammarError { detail, at }` built by `format!` at fourteen
sites in `domain` and seven in `transload`; the position is a byte offset into
whatever string the layer happened to be given. There is no notion of "the
construct expected here", so a refusal can only ever be the sentence somebody
wrote for that branch.

A parser-combinator library is the standard answer to this shape of code. The
grammar is written once as composable values, escapes become one `escape`
parser reused everywhere, positions are spans the library tracks, and an error
carries what was expected at the point of failure. `chumsky` is the one chosen
here: pure Rust, MIT, `rust-version` 1.65 against the workspace's 1.98.1,
`&str` input with `char` tokens and byte-offset spans, and an error type
(`Rich`) that holds labels in the grammar's own vocabulary and accepts custom
messages at any span. Three transitive dependencies (`hashbrown`,
`unicode-ident`, `unicode-segmentation`), two of them already in the tree.

This is a reimplementation of the current grammar, not a dialect change. The
emitters stay as they are, byte for byte, and the acceptance test is that
every byte they write still reads back to the value it was written from.

## What Changes

- **The inline grammar** (`domain::syntax::inline`) parses with a `chumsky`
  parser over `&str`. One `escape` parser replaces the three decoders; each
  construct (`mention`, `bracketed`, `verbatim`, `braced`, `paired`) is a
  named parser; nesting depth is a parameter with a fixed ceiling, so the
  parser is total — it returns a value or a refusal for every input and never
  recurses past the ceiling. Public entry points (`parse`, `parse_with`,
  `Links`, `LinkName`, `Verbatim`) keep their signatures. `GrammarError` keeps
  `detail` and `at` and gains `line` and `column`.
- **The block grammar** (`domain::syntax::block`) likewise. The trailing
  attribute list becomes a forward grammar anchored at the end of the line
  instead of a reverse scan; attribute tokens, quoted values, table rows and
  cells are parsers. `parse`/`parse_with` keep their signatures; `Rendered`,
  `Children`, `INDENT`, `TABLE_ROW`, `CONTAINER_COMPONENTS` are unchanged.
- **The body walker** (`transload::ingest`'s `parse_region`/`parse_node`)
  becomes a grammar over the Landmark file's body with the line prefix (two
  spaces, `> `, or a mix) as an ordinary Rust parameter. Child regions are
  parsed in place as slices of the file, so every refusal's span is a span
  into the file and `Location::line` is read from it rather than carried in a
  tuple. `identity::parse` becomes a parser the walker composes;
  `matching_closer` disappears because a component's body is parsed as the
  nodes it holds.
- **Frontmatter** stops depending on `saphyr`. A small `Node` type — the YAML
  subset the emitter writes, nothing more — sits between the `Frontmatter`
  struct and the text: emit builds a `Node` and writes it (byte-identical to
  today), parse reads a `Node` with a depth-parameterised grammar and
  interprets it against the Chart's `FeatureKind`s exactly as `reading` does
  now. Files outside the subset are refused with the line named. This is the
  one place the change narrows acceptance, and only for hand-edited files;
  design.md D9 says why it belongs here and what the subset is.
- **Refusals** are at least as specific as today's. Inline and block errors
  carry line and column; document-layer errors carry the file line of the
  offending construct, not of the block it sits in; frontmatter errors carry a
  line for the first time. Every refusal names the expected construct in the
  dialect's vocabulary — the sentences the hand parsers write today are kept
  verbatim where tests or people have come to know them (`a bracketed run
needs a `(url)`or`{tokens}` suffix; a literal bracket is escaped`,
  `unclosed `*`, ` `x` is not a mark token in this dialect ``,
  `expected a `wp:mark` comment`, …); no refusal ever shows a combinator or a
  library type name.
- **Parity is tested exhaustively.** A table-driven test per layer walks every
  token, shortcut, escape rule, block form, attribute key and reading kind the
  emitter can write and asserts `parse(emit(v)) == v`. The emitter's own
  source is the checklist.
- **The port is differential.** Each hand-written parser moves to a private
  `legacy` module under `cfg(test)` while its replacement lands; a harness
  parses the committed `getting-started` Bundle, the parity fixtures, the
  conformance suite's generated stores and a list of malformed lines with
  both and asserts the same accept/refuse outcome and the same value. A
  disagreement is a bug in one of them, fixed so they agree. The legacy
  modules are deleted in the final commits.
- **Dependencies.** `chumsky 0.13` enters the workspace table with
  `default-features = false, features = ["std"]`; the `syntax` feature of
  `domain` becomes `["dep:chumsky"]`. `pulldown-cmark`, which that feature
  declares today and no module imports, leaves. `saphyr` leaves `transload`.

## Capabilities

### New Capabilities

_None._ No new spec domain and no new leaf: the dialect grammar is already
`transload`'s business (`openspec/config.yaml`: "the Bundle format — export,
ingest, dialect grammar · code: transload, domain (syntax)").

### Modified Capabilities

- `transload`: gains four requirements — parsing is total, a refusal names the
  construct and its position, every byte the emitter writes reads back, and
  frontmatter is the strict YAML subset the emitter writes. No existing
  requirement changes text; `Ingesting an export reproduces the store` and
  `Export output is byte-stable` are the invariants this change is measured
  against.

## Non-goals

- Changing the dialect — any spelling, any escape rule, any normalization.
  `dialect.md` is edited only to record the nesting ceiling (design.md D4).
- Changing an emitter. Every emitter in `inline.rs`, `block.rs`,
  `frontmatter.rs` and `export.rs` writes the same bytes after this change;
  `frontmatter::emit`'s implementation moves onto `Node` but its output does
  not move by a byte.
- Error recovery. A block with two faults reports its first, as today; ingest
  already collects one refusal per block and per file. `recover_with` stays
  unused.
- Parsing anything `transload` does not parse today: `manifest.json`,
  `_schema.json` and the `.jsonl` sidecars stay on `serde_json`.
- A grammar export, railroad diagram or any generated documentation of the
  grammar.
- Performance work. The hand parsers are not slow and the replacements need
  only not be measurably slower on the conformance suite.
- `cartograph`, the app, or any consumer other than `transload`: nothing else
  enables `syntax`.

## Impact

- `crates/domain`: `syntax/inline.rs` and `syntax/block.rs` parse paths
  rewritten; emitters, token tables (`TOKEN_ORDER`,
  `SHORTCUTS_WITH_INTERIOR_PARSING`, `ALWAYS_ESCAPED`), `coalesce` and tests
  untouched except for new tests.
  `GrammarError` gains two public fields (`line`, `column`) and its `Display`
  reads `… (line L, column C)`; only `domain` constructs it, `transload` only
  formats it. `Cargo.toml`: `syntax = ["dep:chumsky"]`, `pulldown-cmark`
  removed.
- `crates/transload`: `ingest.rs` loses `parse_region`/`parse_node`/`join`/
  `matching_closer`/`body_lines` to a new `document.rs`; `identity.rs` exposes
  its grammar as a parser and keeps `parse` as a wrapper; `frontmatter.rs`
  gains `Node` and loses `saphyr`; `frontmatter::parse` returns a located
  error so `ingest` can cite a line. `Cargo.toml`: `saphyr` removed.
- Workspace `Cargo.toml`: `chumsky = { version = "0.13", default-features =
false, features = ["std"] }` added; `pulldown-cmark` and `saphyr` removed.
  `.github/renovate.json5`: a package rule gating `chumsky` on the dependency
  dashboard, because every `0.x` minor is a breaking release on a roughly
  six-month cadence (design.md D11).
- Dependencies: `chumsky` (MIT) plus `unicode-segmentation` (MIT/Apache-2.0)
  are new in the lockfile; `hashbrown` and `unicode-ident` are already
  present. `cargo deny` passes without a new allowance. `stacker` is not
  pulled: the grammar bounds its own recursion (design.md D4).
- Compile time: `chumsky` is generics-heavy; the grammar is boxed at module
  boundaries so each layer monomorphises once. The before/after delta for a
  cold `cargo build -p transload` is measured in the first and last tasks and
  recorded in design.md; the budget is ten seconds of debug build.
- MSRV: `chumsky` declares 1.65, edition 2021; the workspace is 1.98.1,
  edition 2024. No toolchain change.
- Public surface: no function signature in `domain::syntax` changes.
  `GrammarError` is the one type that grows. `transload`'s public API is
  unchanged; the fuzz entry the pending conformance work may want
  (`parse_region` behind a `fuzzing` feature) is a one-line wrapper over the
  new `document::parse`.
- Tests: `inline.rs`, `block.rs`, `tests/ingest.rs` and `tests/conformance.rs`
  pass unchanged. New: parity tables per layer, differential harnesses (deleted
  with the legacy modules), a `Node` round-trip property, and refusal fixtures
  asserting position and wording.
- Documentation: `dialect.md` gains one sentence on the nesting ceiling. The
  `pulldown-cmark` remark in that document's "Character shortcuts" section
  ("the parser cannot take `pulldown-cmark`'s merged `Emphasis`/`Strong` events
  at face value") already describes a parser that does not exist; it is
  removed in the same edit.
