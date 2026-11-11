## Context

See proposal.md — Why. The constraints the design works within:

- `dialect.md` is normative; `inline.rs`, `block.rs`, `ingest.rs` and
  `frontmatter.rs` are the implementation; the emitters are the oracle. The
  emitter never writes a nested construct — a segment with several marks or a
  link is written `[escaped text]{tokens}` with its interior escaped — so
  every emitted inline construct is one level deep, every emitted block is one
  attribute list, and the parsers accept strictly more than the emitters
  write (nested shortcuts, any token order, `key=value` without quotes).
- Acceptance gates that exist today: unit tests in `inline.rs` and `block.rs`
  (`canonical_sources_round_trip`, `stores_survive_a_trip_through_syntax`,
  `every_content_variant_survives_its_block_form`, `tables_survive_in_both_spellings`,
  `malformed_inlines_are_refused_with_a_location`, …),
  `crates/transload/tests/ingest.rs`, and `tests/conformance.rs`, whose
  `export_is_byte_stable_and_a_fixed_point_of_ingest` is the whole-system
  statement of "every byte the emitter writes reads back". The committed
  `getting-started` Bundle under `crates/store/src/collections/` is a real
  corpus.
- Two test assertions pin refusal content: `inline.rs` asserts a `Links`
  error's `detail` passes through verbatim (`"no page `Nowhere`"`), and
  `malformed_inlines_are_refused_with_a_location` asserts `at <= source.len()`.
  `tests/ingest.rs` asserts `at.line.is_some()` for identity and sidecar
  refusals.
- `GrammarError { detail, at }` is public; `transload` formats it and never
  constructs it. `MalformedKind::Grammar { detail }` plus
  `Location { file, line }` is the document layer's refusal.
- Per-commit hooks (`.config/hk.pkl`): `clippy --workspace --all-targets`,
  `nextest` over a computed filterset, `deny`, `treefmt`, `typos`, `lychee` on
  markdown. The workspace lints deny `todo!` and `dbg!` and warn on
  `uninlined_format_args`.
- `chumsky` 0.13.0 (2026-05-06): `Parser<'src, I, O, E = extra::Default>`,
  `&'src str` as `Input` with `Token = char`, `Cursor = usize`,
  `Span = SimpleSpan` over byte offsets; `Rich<'a, T, S = SimpleSpan,
C = String>` with `custom(span, msg)`, `span()`, `reason()`, `expected()`,
  `map_span`; `RichReason::{ExpectedFound { expected, found }, Custom(C)}`;
  `RichPattern` is `#[non_exhaustive]`; `Parser::{labelled, try_map,
map_with, validate, boxed, and_is, not, or_not, repeated, to_slice}`;
  `MapExtra::{span, slice, state, ctx, emit}`;
  `ParseResult::{into_result, into_output_errors}`. Default features are
  `std` and `stacker`; non-optional dependencies are `hashbrown`,
  `unicode-ident`, `unicode-segmentation`. These were read from the 0.13.0
  documentation; the code sketches below are untested.
- Releases: 0.10.0 (2025-03), 0.11.1 (2025-09), 0.12.0 (2025-12), 0.13.0
  (2026-05). Each `0.x` minor is a breaking release; the `1.0.0-alpha.*`
  line (2023–2025-01) preceded 0.10 and is dormant.

## Goals / Non-Goals

**Goals:**

- Same language. `parse(emit(v)) == v` for every value the emitter accepts,
  proven per layer by an exhaustive parity table and end to end by the
  conformance suite; the committed Bundle ingests to the same store.
- Total parsers. Every entry returns a value or a refusal for any input,
  never panics, never fails to terminate, never exceeds a fixed recursion
  depth.
- Refusals at least as specific as today's: line and column within the
  source given (inline, block), the file line of the construct (document),
  the line (frontmatter); the expected construct named in the dialect's
  vocabulary; today's sentences kept.
- One escape parser, one token grammar, one attribute grammar, one line
  grammar — each written once.
- Emitters untouched; `frontmatter::emit` moves onto `Node` without a byte
  changing.

**Non-Goals:**

- Error recovery, multiple errors per block, or diagnostics beyond the first
  refusal.
- Any dialect change, including "cheap" leniencies the hand parsers do not
  have.
- Performance beyond parity with the hand parsers on the conformance suite.
- A grammar export of any kind.

## Decisions

### D1 — A reimplementation gated by equality, not a cleanup (2026-11-01)

The parse paths are rewritten whole, one layer per commit, with the
hand-written code kept beside them as a test oracle until the last commit.
The gate for each layer is (a) its existing tests, (b) a parity table that
enumerates the emitter's branches, and (c) a differential harness over
corpora that asserts the old and new parser agree. Nothing is "improved" on
the way through: a disagreement is a bug in one of the two, and the fix lands
in whichever is wrong so that they agree again. When the hand parser is the
wrong one, its fix is its own commit before the port continues, so the
differential stays exact.

_Rejected:_ cleaning the hand parsers in place (one escape helper, a span
type). It removes duplication but leaves the shape the proposal objects to —
positions and expectations are still something each branch assembles by
hand. _Rejected:_ porting only the inline layer. The block and document
layers hold the same scanners (`split_attrs`, `split_cells`,
`matching_closer`) and the document layer is where positions are worst.

### D2 — `chumsky` 0.13 over `&str`, with `Rich<char>` errors and no `stacker` (2026-11-01)

```toml
# Cargo.toml (workspace)
chumsky = { version = "0.13", default-features = false, features = [ "std" ] }
```

Every grammar in this change is `Parser<'src, &'src str, O, Err<'src>>` with
`type Err<'src> = extra::Err<Rich<'src, char>>`. `&str` input keeps spans as
byte offsets into the string a caller handed over, which is what
`GrammarError.at` already is, and `char` tokens let a refusal name the
character found. `Rich` is the only error type with labels and custom
messages; `Simple` and `Cheap` cannot carry the dialect's vocabulary.

`stacker` (a default feature) grows the stack onto the heap when a recursive
parser runs deep. It is not taken: it pulls `psm`, a crate with a `cc`-built
assembly shim, onto eleven cross targets, and D4 bounds every recursion in
this change so the stack never needs to grow. `std` is kept for `Boxed`'s
`Rc` and the `String` message type.

_Rejected:_ `features = ["stacker"]` — a native build step to support a
nesting depth nobody writes, and totality still unproven since the heap spill
is unbounded. _Rejected:_ the `label` of a `&[Line]` token input for the
document layer (lines as tokens, chumsky over a slice) — it would make the
walker a two-pass lexer-plus-parser and lose the file-relative spans D8 gets
for free.

### D3 — Openers commit; refusals are emitted, not failed (2026-11-01)

The hand parsers have one property a naive combinator grammar loses: once an
opener is seen, the construct either parses or _refuses_ — `[[` with no `]]`
is an error, never the literal text `[[`. `choice` in chumsky has no such
commitment; a failing alternative falls through to the next, and the last
alternative is "one literal character". Two rules restore the behaviour:

1. **Literal text excludes opener characters.** The literal parser is
   `escape | none_of(OPENERS) | lone_tilde_equals_pipe | caret_not_before_brace`,
   where `OPENERS` is `[`, `` ` ``, `$`, `*`, `_` (the always-escaped set
   minus `\` and `]`), `~`/`=`/`|` are literal only when not doubled, and `^`
   only when not followed by `{`. Nothing that can open a construct is
   accepted as text, so there is nothing to fall through to.
2. **Refusals are `validate`d, not `try_map`ped.** A construct that has
   consumed its opener always _succeeds_ as a parser; what it cannot find it
   reports through `MapExtra::emit` (or `validate`'s emitter) as
   `Rich::custom(span, sentence)` at the span the dialect would point at — the
   opener for `unclosed …`, the token for `` `x` is not a mark token ``, the
   name for `` `::x` is not a block component ``. `ParseResult` then carries
   errors alongside an output and `into_result` refuses. The first emitted
   error is the one reported; emission order is depth-first, which is the
   order the hand parser's `?` reports in (an unclosed inner shortcut before
   its unclosed outer one).

`try_map` is still used where failure is the right answer — inside
`choice` arms that have not yet consumed an opener, and in the document layer
for "this line is not an identity comment". `.labelled(…)` names constructs
for the residual `ExpectedFound` cases (a `wp:mark` line expected, end of
input where a closer was) so the mapping in D6 never has to invent a
sentence.

_Rejected:_ `try_map` everywhere — inside `or_not()` and `choice` it
produces fall-through, which is an acceptance change the differential harness
would flag on its first run (`[hi]{nonsense}` would parse as text).
_Rejected:_ `recover_with` — recovery is for continuing past a fault to find
more; ingest refuses the file, so there is nothing to continue to.

### D4 — Depth is a Rust parameter with a ceiling; no `recursive` in the inline layer (2026-11-01)

Every recursive grammar takes its depth (or, for the walker, its line
prefix) as an ordinary function parameter and refuses past a fixed ceiling:

| layer       | parameter                    | ceiling | refusal                                                  |
| ----------- | ---------------------------- | ------- | -------------------------------------------------------- |
| inline      | `depth: usize`               | 16      | `inline marks nest deeper than 16 levels`                |
| document    | `prefix: String` (per level) | 256     | `the body nests deeper than 256 levels`                  |
| frontmatter | `depth: usize`               | 4       | `frontmatter nests deeper than a Landmark's readings do` |

_Amended 2026-11-02, while implementing task 3.1._ The inline layer does not
build a parser per depth. A run's depth and the closer that ends it ride in
chumsky's parse context (`extra::Full<_, _, Nest>`): a construct computes
`Nest { depth + 1, closer }` with `map_with` and runs the one `recursive` run
parser under it with `ignore_with_ctx`. Context is a value handed down, not
state, so backtracking cannot corrupt it, and construction is one recursive
declaration per `parse_with` call instead of one boxed parser per depth and
closer (65 × 9). The run checks its depth before reading anything and, past
the ceiling, consumes the rest of the input without descending, so the
recursion is bounded by the ceiling. Nine mutually recursive
`Recursive::declare` runs were rejected: a declared `Recursive` clones as a
strong `Rc`, so mutual references leak a cycle on every call.

The inline ceiling is 16, not 64. Measured on aarch64-darwin, a debug build
spends 32–64 KiB of stack per nesting level (chumsky's `#[inline(always)]`
combinators collapse into large unoptimised frames): depth 16 needs 0.5–1 MiB
and depth 25 needs 1–1.5 MiB, so 64 cannot fit a two-megabyte thread. A
release build parses past the ceiling in under 256 KiB. Sixteen keeps a debug
parse at the ceiling within half of a two-megabyte thread and still sits far
above anything the emitter or a person writes.

Depth here is syntactic: how many openers are unclosed at one point in the
_source_ (`^{^{^{…`, `[[[…`, `> > > …`, callouts inside callouts), not how
many marks a `RichText` carries or how a store is shaped. Re-opening a kind
that is already open is legal text that `wrap` turns into a no-op, so the
source can nest without bound while the value it parses to cannot. Canonical
output never comes near either ceiling: the emitter writes one level per
mark kind and the app's outline stops loading children at twenty. The
ceilings exist for the other input — hand-written, corrupted, or fuzzed —
because the spec says parsing is total, and a parser with no ceiling is only
total up to its stack. The hand parsers have none; they recurse per nesting
level and would exhaust the stack on `^{^{^{…` a few thousand deep. The
ceiling turns that abort into a refusal. The values need only sit well above
anything legitimate and well below the stack; `dialect.md` records the inline
one in a sentence so the limit is a documented property of the dialect's
_parser_, not an accident.

_Rejected:_ `recursive(|run| …)` with `stacker` — unbounded recursion made
safe by heap-allocated stacks; see D2. _Rejected:_ a depth counter in
`extra::State` — state mutations are not undone on backtrack, so a failed
alternative leaves the counter wrong. _Rejected:_ no ceiling — the fuzz
targets the pending conformance work proposes feed 4 KiB inputs, which is two
thousand `^{` levels, and a tokio worker thread has a two-megabyte stack.

### D5 — `Links` is captured by the parser constructor (2026-11-01)

`inline::parser(links: &dyn Links)` builds the grammar with `links` captured
by the closures that need it (`mention`'s `validate`), and `parse_with`
builds and runs it per call. Both references are shortened to one lifetime,
so the signature is plainly
`fn parser<'a>(links: &'a dyn Links) -> Boxed<'a, 'a, &'a str, RichText, Err<'a>>`.
The `Links::resolve` error string becomes the refusal's `detail` unchanged,
which is what `named_mentions_escape_the_page_and_elide_a_title_label`
asserts.

Every other semantic check lives the same way: `parse_token` runs inside the
attribute list's `validate`, `color_from` inside the theme token's, the
component-name match inside `component`'s. The oracle stays serde (`Color`
deserialisation) exactly as today, so the grammar never lists colours.

_Rejected:_ `extra::State<&dyn Links>` with `parse_with_state` — it works,
but `State` is for mutable parse state and threads a type parameter through
every combinator for a read-only reference. _Rejected:_ `extra::Context` —
chumsky's context is for context-sensitive grammars (`with_ctx`,
`configure`), not an environment. _Rejected:_ caching the built parser —
`Boxed<'src, …>` is tied to the input lifetime, and construction is cheap
enough (D4) that a cache buys nothing measurable.

### D6 — Mapping `Rich` to `GrammarError` and `Malformed` (2026-11-01)

```rust
// domain::syntax::inline — sketch, untested
pub struct GrammarError {
    pub detail: String,
    /// Byte offset into the source handed to `parse`.
    pub at: usize,
    /// 1-indexed, within that source.
    pub line: usize,
    /// 1-indexed, in characters, within `line`.
    pub column: usize,
}
// Display: "{detail} (line {line}, column {column})"

fn locate(src: &str, offset: usize) -> (usize, usize) {
    let before = &src[..offset.min(src.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, column)
}

pub(crate) fn to_grammar(src: &str, error: &Rich<'_, char>) -> GrammarError {
    let at = error.span().start;
    let (line, column) = locate(src, at);
    let detail = match error.reason() {
        RichReason::Custom(sentence) => sentence.clone(),
        RichReason::ExpectedFound { expected, found } => {
            let expected: Vec<String> = expected.iter().map(pattern).collect();
            let found = match found {
                Some(c) => format!("`{}`", **c),
                None => "the end of the text".to_string(),
            };
            format!("expected {}, found {found}", expected.join(" or "))
        }
    };
    GrammarError { detail, at, line, column }
}

fn pattern(p: &RichPattern<'_, char>) -> String {
    match p {
        RichPattern::Label(label) => label.to_string(),
        RichPattern::Token(c) => format!("`{}`", **c),
        RichPattern::EndOfInput => "the end of the text".into(),
        // `RichPattern` is non-exhaustive; anything else is a dialect construct
        // that forgot its label, which the refusal fixtures catch.
        _ => "something else".into(),
    }
}
```

Rules the mapping holds to:

- `Custom` sentences are the hand parsers' sentences, kept verbatim: every
  `format!` at an `error(…)`/`grammar(…)` site today becomes the message of
  one `validate` emission. The refusal fixtures list them.
- `ExpectedFound` only ever renders labels (`a `wp:mark` comment`, `a
closing `]]``) or a quoted character; no `RichPattern::Identifier`, no
  `Debug` of a chumsky type, no `SimpleSpan` in text.
- The document layer maps a `Rich` whose span is into the file straight to
  `Location::line(path, line_of(span.start))` with `detail` the sentence
  alone. For a block refusal that came through `block::parse_with` over the
  block's own lines, the file line is `first_own_line + error.line - 1` and
  `detail` is `format!("{} (column {})", error.detail, error.column +
prefix_width)` — a real line and a real column, where today there is the
  block's first line and a byte offset into a joined string.
- Frontmatter refusals are `Location::line(path, 1 + line)` (the opening
  fence is line 1), where today they are the whole file.

### D7 — The block layer: a forward grammar for the trailing attribute list (2026-11-01)

`split_attrs` scans backwards because the attribute list is the _last_
`{…}` on the line and the text before it may contain braces (`set x = {1}{}`,
`[hi]{bold}{theme-blue}`). The same fact reads forwards as "a brace group
that runs to the end of the line and contains no unquoted `{`":

```rust
// domain::syntax::block — sketch, untested
fn attrs_at_end<'a>() -> impl Parser<'a, &'a str, Attributes, Err<'a>> {
    let quoted = just('"')
        .ignore_then(choice((just('\\').ignore_then(any()), none_of('"'))).repeated().collect::<String>())
        .then_ignore(just('"'));
    let bare = none_of(" \t\"{}").repeated().at_least(1).to_slice();
    let token = choice((bare.then(just('=').ignore_then(quoted.clone().or(bare.map(str::to_string))).or_not()),))
        .map_with(|(key, value), e| (key, value, e.span()));
    just('{')
        .ignore_then(token.padded_by(text::inline_whitespace()).repeated().collect::<Vec<_>>())
        .then_ignore(just('}'))
        .then_ignore(text::inline_whitespace())
        .then_ignore(end())
        .validate(|tokens, _, emitter| interpret(tokens, emitter)) // theme-… via Color, key="value", else refuse at the token's span
}

fn attributed_line<'a>() -> impl Parser<'a, &'a str, (&'a str, Attributes), Err<'a>> {
    any().and_is(attrs_at_end().not()).repeated().to_slice()
        .then(attrs_at_end().or_not().map(Option::unwrap_or_default))
}
```

The text half is the slice before the list — whitespace preserved exactly as
`split_attrs` preserves it (`"set x = {1} "` keeps its trailing space; a line
with no list is returned whole, trailing whitespace included). The interior's
quoting rule is the hand parser's (`split_tokens`: whitespace splits outside
quotes, a backslash inside quotes escapes the next character); `interpret` is
`parse_attrs` as a `validate` body so an unknown token refuses at its own
span rather than at the opening brace.

Everything else in the block layer is a straight transcription: the first
line dispatches on ` ``` `, `$$`, `::`, `| `, `>`, `### `/`## `/`# `, `- `,
`+ `, then `---` or paragraph; fenced bodies take lines until a line that
trims to the fence; table rows are `| ` lines split into cells by an unescaped
`|`, with `\|` restored, cells trimmed, and an odd trailing backslash run
read as an escaped trailing space. The block parser stays prefix-free and
standalone — `block::parse(source)` is the same entry the tests, the fixtures
and any fuzz target call — and the document layer (D8) hands it a block's own
lines with their prefix stripped, as `ingest.rs` does today.

The leniencies the hand parser has are kept because the differential harness
would otherwise flag them: a fenced block whose closer is missing takes every
remaining line (the walker refuses unclosed fences before the block parser
sees them); `>text` after the quote marker is ignored; an unparsable
`highlights` entry is dropped; `key=value` without quotes is accepted;
`col-header` written explicitly wins over the delimiter-row inference.

_Rejected:_ locating the split with the reverse scan and parsing only the
halves with chumsky. It keeps the one scanner the proposal names first.
_Rejected:_ making the block parser prefix-aware so the walker could parse
blocks in place. It would give block refusals file spans directly, but it
puts the walker's layout concern into `domain` and changes the standalone
entry's grammar; line arithmetic in the walker is two additions.

### D8 — The document walker: prefix as the parameter, child regions parsed in place (2026-11-01)

_Amended 2026-11-02, while implementing task 4.2._ Nesting in the body is
plain Rust recursion, not chumsky recursion. Inline nesting is what a person
types; body nesting is what a store holds, and the exporter writes it at any
depth, so the 256 ceiling has to hold for real outlines. Measured on the
inline layer, chumsky recursion costs 32–64 KiB of stack per level in a debug
build; 256 levels would need several megabytes. The walker therefore keeps a
recursion of its own (`region` → `node` → `region`), with the heavy reading in
a non-recursive `plan` step so each level costs only a small frame: a body
nested 256 levels parses on a two-megabyte thread in a debug build, and one
nested 257 refuses. What D8 asked of it otherwise holds. Every level reads the
file's lines through a `View` (the line past its prefixes, with the prefix
width kept for columns) instead of copying them; the lines, identity comments
(`identity::marker`), and each block's opener shape are grammars; and a block
refusal cites the file line of the construct and a column that counts its
prefixes. `matching_closer` survives as `closer`, unchanged, because a
component's extent must match the hand walker's depth count exactly (the
differential holds it to that). Identity refusals stay `MalformedKind` values
rather than emitted errors, since `SourceKeyedIdentity` is a kind, not a
sentence.

The body of a Landmark file is a sequence of nodes at a _prefix_ — `""` at
the root, `"  "` under an indented parent, `"> "` under a blockquote, and
concatenations of those under nested parents. A node is an identity comment,
the block's own lines, optionally a child region at a deeper prefix, and —
if there were children — a closing comment. The emitter writes a blank line
between siblings and writes an empty child line as the prefix trimmed (`>`,
or nothing for an indent).

```rust
// transload::document — sketch, untested
type Err<'a> = extra::Err<Rich<'a, char>>;

fn newline<'a>() -> impl Parser<'a, &'a str, (), Err<'a>> + Clone {
    just('\r').or_not().then(just('\n')).ignored().or(end())
}

/// One line wearing `prefix`; yields its content. A line that is only the
/// trimmed prefix is an empty content line (the emitter writes `>` and `` for those).
fn line<'a>(prefix: &str) -> impl Parser<'a, &'a str, &'a str, Err<'a>> + Clone {
    let full = just(prefix.to_string()).ignore_then(none_of('\n').repeated().to_slice());
    let blank = just(prefix.trim_end().to_string()).to("");
    choice((full, blank)).then_ignore(newline())
}

/// Sibling nodes at `prefix`. Same-prefix recursion (a callout's body) goes
/// through `recursive`; deeper prefixes go through `children`, which parses
/// the child slice with a fresh `region` built for the deeper prefix.
fn region<'a>(prefix: String, links: &'a dyn Links, depth: usize)
    -> Boxed<'a, 'a, &'a str, Vec<Node>, Err<'a>>
{
    recursive(|region| {
        let blank = line(&prefix).filter(|content| content.trim().is_empty());
        let node = node(prefix.clone(), links, depth, region);
        blank.clone().repeated().ignore_then(node).then_ignore(blank.repeated()).repeated().collect()
    })
    .boxed()
}

/// The child lines beneath a node, as a slice of the file, reparsed at the
/// deeper prefix. The span offset keeps every refusal relative to the file.
fn children<'a>(deeper: String, links: &'a dyn Links, depth: usize)
    -> impl Parser<'a, &'a str, Vec<Node>, Err<'a>>
{
    let child_line = line(&deeper).filter(|c| !c.trim().is_empty());
    let blank_run_then_child = line("").filter(|c| c.is_empty()).repeated().at_least(1).then(child_line.clone());
    choice((child_line.ignored(), blank_run_then_child.ignored())).repeated().at_least(1).to_slice()
        .try_map(move |slice: &'a str, span| {
            if depth + 1 > MAX_DEPTH {
                return Err(Rich::custom(span, format!("the body nests deeper than {MAX_DEPTH} levels")));
            }
            let base = span.start;
            region(deeper.clone(), links, depth + 1).parse(slice).into_result().map_err(|errors| {
                errors.into_iter().next().expect("a refused parse has an error")
                    .map_span(|s| (s.start + base..s.end + base).into())
            })
        })
}
```

`node(prefix, …)` is `line(prefix)` through `identity::marker()` (an `Open`
or the refusal `expected a `wp:mark` comment`), then the self-link check
(`inline::parse_with` over the next line, one segment, a mention of this id —
unchanged), then one of: a fence run (` ``` `…` ``` ` or `$$`…`$$`, with
`unclosed block, expected `…`` emitted at the opener if the closer never
comes), a `::` opener whose name is in `CONTAINER_COMPONENTS` (own line only;
the body is `region.clone()` at the same prefix; then the `::` closer, or
`unclosed `::` component`), any other `::` opener (own lines through the
closer — rows for `::table`, a refusal for an unknown name once the block
parser sees it), a run of `| ` lines, a `>` line (own line if it opens `>{`,
otherwise `>` with no own line; children at `prefix + "> "`), or one line.
Then `children(prefix + "  ", …)` for indented kinds, and if children came,
`line(prefix)` through `identity::marker()` expecting `Close` or
`a container's comment is never closed`. The node's `line` is computed from
its span start with a line index built once per file. `matching_closer` has
no successor: a `::` inside a fenced body is consumed by the fence node.

The two entries `parse_region` and `parse_node` and the `Region` alias go;
`document::parse(body: &str, links) -> Result<Vec<Node>, Malformed-shaped
error>` replaces them, and `body_lines` becomes a slice taken after
`frontmatter::split`. The outline pass (`Lenient`) calls the same function.

_Rejected:_ keeping the `Vec<(usize, String)>` walker and parsing only
single lines with chumsky — it keeps the copying and the joined-string
offsets. _Rejected:_ `extra::Context` for the prefix — it is the designed
tool for indentation-sensitive grammars in chumsky, but the maintainer's
direction is a plain parameter, and the slice-and-reparse step makes the
depth bound (D4) a local check rather than a property of a context type.
_Rejected:_ an eager `region(prefix)` that constructs its deeper regions —
the prefix branches on node kind (`"  "` or `"> "`), so eager construction
is exponential in the ceiling.

### D9 — Frontmatter is in this change, last, as a strict subset over a `Node` pivot (2026-11-01)

Frontmatter is the Bundle's own grammar, not the dialect's, and it is the one
layer where this change changes acceptance: hand-edited files using YAML the
emitter never writes (tags, anchors, single quotes, multi-line flow, nested
flow beyond the icon) are refused after it. It is here rather than a
follow-up because (a) `saphyr` out and `chumsky` in is one dependency
decision — leaving `saphyr` would leave `transload` with two parsing stacks
after a change whose point is one; (b) the `Node` pivot makes
`parse(write(node)) == node` a property in the same family as the dialect
layers' parity tables, tested with the same machinery; (c) the depth
parameter is the technique D4 and D8 already establish. It is _last_ so the
zero-change layers land and prove themselves first, and a maintainer who
wants the narrowing reviewed separately can stop before section 5 of
tasks.md without losing anything.

```rust
// transload::frontmatter — sketch, untested
/// The YAML subset the emitter writes. Emit builds one and writes it; parse
/// reads one. Styles are kept so the round trip is exact.
pub(crate) enum Node {
    /// A bare scalar: ids, `null`, `true`, numbers, timestamps.
    Plain(String),
    /// `"…"` with `\\`, `\"`, `\n` as the only escapes.
    Quoted(String),
    /// `|-` / `|2-`, strip-chomped; lines one step deeper than the key.
    Literal(String),
    /// One line of compact JSON, as `serde_json` writes it (the icon).
    Json(serde_json::Value),
    /// `[a, b]` — scalars separated by `, ` (a phone reading).
    Flow(Vec<Node>),
    /// `- item` lines one step deeper, or `[]` when empty.
    Seq(Vec<Node>),
    /// `key: value` lines, or `{}` when empty.
    Map(Vec<Entry>),
}

pub(crate) struct Entry {
    pub key: String,
    pub value: Node,
    /// A trailing `# …` on the key line. Advisory: written, read, never interpreted.
    pub note: Option<String>,
}

/// A writer helper, not a variant: an id is `Plain` unless YAML would re-type
/// it (`0001`, `12e3`, `null`, `true`, `false`, `~`), when it is `Quoted`.
impl Node { pub(crate) fn id(text: &str) -> Node { … } }

fn map<'a>(depth: usize) -> Boxed<'a, 'a, &'a str, Vec<Entry>, Err<'a>> {
    if depth > MAX_DEPTH {
        return empty().validate(|_, e, emitter| {
            emitter.emit(Rich::custom(e.span(), "frontmatter nests deeper than a Landmark's readings do"));
            Vec::new()
        }).boxed();
    }
    let indent = just("  ".repeat(depth));
    let key = choice((quoted_scalar(), none_of(":#\"\n").repeated().at_least(1).to_slice().map(str::to_string)));
    let note = text::inline_whitespace().ignore_then(just('#')).ignore_then(none_of('\n').repeated().to_slice())
        .map(|n| n.trim().to_string()).or_not();
    let on_line = just(' ').ignore_then(choice((flow(), json(), quoted_scalar().map(Node::Quoted), plain_scalar())))
        .then(note.clone()).then_ignore(newline());
    let literal = just(" |").ignore_then(just('2').or_not()).then_ignore(just('-')).then(note.clone()).then_ignore(newline())
        .then(literal_lines(depth + 1)).map(|((_, note), text)| (Node::Literal(text), note));
    let nested = note.clone().then_ignore(newline())
        .then(choice((seq(depth + 1).map(Node::Seq), map(depth + 1).map(Node::Map))))
        .map(|(note, value)| (value, note));
    let entry = indent.ignore_then(key).then_ignore(just(':')).then(choice((on_line, literal, nested)))
        .map(|(key, (value, note))| Entry { key, value, note });
    let skip = choice((text::inline_whitespace().then(newline()).ignored(), own_line_comment(depth)));
    skip.clone().repeated().ignore_then(entry).then_ignore(skip.repeated()).repeated().at_least(1).collect().boxed()
}
```

What the subset accepts, and the cheap tolerances that come with it:

- Block mappings and sequences at a two-space step; a sequence item is a
  scalar, a flow, or a mapping whose first key shares the `- ` line and whose
  further keys sit two deeper (`- ref: …` / `label: …`).
- Scalars: plain (no leading `"`, `[`, `{`, `|`, `#`; runs to the note or the
  end of the line, trailing whitespace trimmed), double-quoted with exactly
  `\\`, `\"`, `\n`, literal block `|-` and `|2-` strip-chomped (a body line
  shorter than the indent is an empty line), one-line compact JSON (`{"` …),
  flow `[a, b]` of scalars, and the empty flows `[]`, `{}`.
- Comments: a trailing `# …` on a key line is the entry's `note`; an own-line
  `# …` between entries is skipped; both are advisory and nothing reads them.
- Whitespace: spaces after `:` and `-` beyond the one written; trailing
  whitespace on any line; blank lines between entries; CRLF line ends.
- Refused, naming the line: tags (`!foo`), anchors and aliases, single-quoted
  scalars, multi-line or nested flow (other than compact JSON), a `?` complex
  key, document markers inside the frontmatter, tabs as indentation, a step
  that is not two spaces, depth past 4, and any construct the list above does
  not name.

Interpretation (`reading(node, kind)`) is a transcription of today's: a
`Plain("null")` is `Reading::Null`; `Text` reads a scalar's text through
`inline::parse_with`; `Select` versus `MultiSelect` is still decided by the
feature's kind; `label` under a reference is still not read; `created` and
`updated` are read as text and parsed with `bundle::parse_timestamp`. The
`Node` round trip `parse(write(n)) == n` and `write(parse(write(n))) == write(n)`
hold for every `n` the writer produces, and `emit`'s bytes are asserted equal
to the current emitter's over the frontmatter fixtures and the committed
Bundle before `saphyr` is removed.

_Rejected:_ a follow-up change for frontmatter — see above; it would be
specified entirely by this section and would leave `saphyr` in place.
_Rejected:_ `serde_yaml`-family or `saphyr-serde` — dynamic `FeatureID` keys
and the advisory comments are why emit is hand-rolled today, and a serde
reader still accepts all of YAML. _Rejected:_ a `Node::Id` variant — it would
make `"0001"` parse to two different nodes depending on intent; keeping styles
as variants and quoting ids in a writer helper keeps the round trip exact.
_Rejected:_ a formatter or any acceptance of YAML beyond the list — there is
no second writer of frontmatter, and tooling that reads the Bundle reads
YAML, so the subset need only be valid YAML, not all of it.

### D10 — Parity tables enumerate the emitter; a differential harness guards the port (2026-11-01)

**Parity.** One table-driven test per layer, built from the emitter's
source as a checklist, lands before the port and must pass on the hand
parser:

- inline: every `TOKEN_ORDER` token, every `SHORTCUTS_WITH_INTERIOR_PARSING`
  delimiter, every arm of `shortcut` including the four that fall to the span
  form (parameterised highlight, italic opening with `{`, code containing
  `` ` ``, math containing `$`), the `bg-`/`text-`(colour)/`text-`(size)/`font-`
  prefixes, every mention spelling `emit_segment` writes (bare, labelled, block,
  elided title, escaped page), the link, the plain run opening with `{`, every
  `ALWAYS_ESCAPED` character, the doubled and edge cases of `~`/`=`/`|`, `^`
  before `{` and at a run's end, `}` in a script, `)` and `\` in an href;
- block: every `Content` variant themed and unthemed, every attribute key
  (`filename`, `highlights`, `engine`, the seven table keys), the `{}`
  suffix for a brace-ending line, every `LINE_OPENERS` escape, both table
  spellings, every `escape_cell` rule;
- frontmatter: every `Reading` variant empty and non-empty, every `plain`
  re-typing case, the icon, the `|2-` indicator, notes present and absent.

The task that lands each table measures the emitter functions' line coverage
under it with `cargo llvm-cov` and records a gap as a missing row.

**Differential.** During the port the old parse path lives in
`syntax::inline::legacy` / `syntax::block::legacy` / `transload::document::legacy`
/ `frontmatter::legacy` under `#[cfg(test)]`. A harness per layer parses,
with both, every line and block of the committed Bundle, every parity
fixture, every store the conformance suite generates (`TRANSLOAD_SEEDS`),
and a committed list of malformed inputs drawn from
the hand parsers' tests; it asserts equal `is_ok()` and equal `Ok` values.
Error text is not compared — the new one may be more specific — but a
separate fixture asserts, for every sentence in the hand parsers, that the
new parser produces it at the expected line and column. The legacy modules
and harnesses are deleted in the final two commits once the conformance suite
has passed on the new parsers at `10 000` cases.

_Rejected:_ comparing error text — it would pin the port to byte offsets.
_Rejected:_ keeping the legacy modules as a long-term oracle — two parsers is
the problem, not the solution.

### D11 — Pin `0.13`, gate bumps on the dashboard (2026-11-01)

The workspace entry is `chumsky = "0.13"`: Cargo's caret on a `0.x`
requirement already excludes `0.14`, so a routine `cargo update` cannot
cross a breaking release. Renovate's "cargo non-major" group would still
propose `0.14` as a minor; a `packageRules` entry for `chumsky` sets
`dependencyDashboardApproval: true` with a description, in the shape of the
`turso` and `gh-workflow backend` rules already there. Accepting a bump is a
`chore(deps)` commit whose gate is the same as the port's: parity tables,
refusal fixtures, the conformance suite. The budget is a day per bump at the
observed cadence of one breaking minor every six months; the 0.11 → 0.12 →
0.13 changelogs touched combinator names and `extra` types, not grammar
semantics, and every grammar here is a few hundred lines behind `Boxed`
boundaries.

_Rejected:_ `=0.13.0` — it blocks patch releases, which are bug fixes.
_Rejected:_ vendoring or forking — nothing here needs a change to chumsky.

### D12 — Where `.boxed()` goes, and how compile time is measured (2026-11-01)

`Boxed` is used exactly where a type must be named or recursion must stop:
the inline layer's construct choice and its suffix list (cloned into every
construct, so one box each, inside the one `recursive` run; see D4's
amendment), `document::region` (one box per prefix instance, plus
`recursive`'s own), `frontmatter::map` and `seq` (one box per depth). The
block layer needs none: its opener grammar is a single `choice` built per
call. Nothing else is boxed. The public entries build the parser, call
`parse`, map the result, and return — chumsky types never appear in a public
signature.

Compile time is measured with the toolchain's `cargo` (not the `mbx` shim,
which skews `--timings`): `cargo clean -p domain -p transload && cargo build
--timings -p transload`, three runs, before task 1.1 and after task 6.2, on
the same machine; the `domain` and `transload` unit times are what is
recorded in this file. The budget is ten seconds of debug build; exceeding it
is a prompt to box one more boundary, not to abandon the port.

| measured (2026-11-02, aarch64-darwin) | `domain`               | `transload`            |
| ------------------------------------- | ---------------------- | ---------------------- |
| before task 1.1 (hand parsers)        | 2.17 s, 2.12 s, 2.13 s | 2.15 s, 2.13 s, 2.14 s |
| after task 6.2 (chumsky grammars)     | 5.68 s, 4.34 s, 3.69 s | 6.80 s, 4.34 s, 3.64 s |

### D13 — The `syntax` feature gates `chumsky`; nothing else changes (2026-11-01)

`domain`'s `syntax` feature exists so that every dependent except
`transload` pays neither the module nor its dependency. It keeps that role
with `syntax = ["dep:chumsky"]`. `pulldown-cmark`, the dependency it declares
today, is imported by no module in `crates/` — the port removes it from the
feature and from the workspace table in the same commit that adds `chumsky`,
so the feature's dependency is replaced rather than extended.

## Behaviours the port preserves

Found reading the hand parsers; each is a row in a parity or differential
fixture so that it cannot be lost by accident:

- A lone trailing `\` is literal; `\x` is `x` for any `x`, including inside
  `(url)`, `{tokens}`, `[[…]]` and braced scripts; code and math interiors
  take no escapes at all.
- `[[#abcd]]` (empty page, block prefix) is a mention; `[[]]` and `[[|x]]`
  are `a mention with no target`; a second `#` or `|` inside a mention is
  literal; a label is literal text, never parsed for marks; a `{tokens}`
  suffix after `]]` applies to the mention segment.
- Nested shortcuts apply each mark once (`wrap` skips a kind already present)
  and canonicalise to one span on re-emit.
- `split_attrs`: with a list, the text keeps every character before `{`
  including trailing spaces; without one, the whole line including trailing
  whitespace. An escaped `\}` at the end means no list.
- The block parser trusts its caller for closers (unterminated fences take
  every remaining line); `highlights` drops entries that are not numbers;
  `>` followed by text yields a blockquote and ignores the text; an explicit
  `col-header` beats the delimiter-row inference.
- The walker requires exactly two spaces per indented level and `> ` (or a
  bare `>`) per quoted level; a blank line continues a child region only if
  the next non-blank line is still in it; an identity comment is matched
  after `trim`; an ordinary HTML comment where an identity is expected
  refuses with `expected a `wp:mark` comment`; `src`-keyed identities refuse
  as `SourceKeyedIdentity` with a line.
- The outline pass (`Lenient`) and the full pass parse the same body with
  different `Links`; a file the outline cannot parse is reported by the full
  pass.

## Risks / Trade-offs

- [A disagreement between old and new on an input nobody writes] → the
  differential harness treats it as a bug and the fix lands on whichever side
  is wrong before the port continues (D1). The cost is a few extra commits;
  the benefit is that the hand parser's accidental grammar is either
  documented in a fixture or corrected, never silently replaced.
- [`chumsky` breaking minors every six months] → caret pin plus dashboard
  approval (D11); grammars are small and boxed; the port's own test suite is
  the migration test. If a bump is ever more than a day, the dialect is
  stable enough that staying on the prior minor costs nothing.
- [Compile time] → measured (D12), boxed at boundaries, budgeted at ten
  seconds; only `transload` builds the module.
- [Frontmatter narrowing refuses a hand-edited Bundle that ingested before]
  → the refusal names the line and what the subset expected; the subset is
  exactly the emitter's output so an exported Bundle is unaffected; and a
  Bundle is a snapshot format, not a hand-authoring one (D9).
- [Nesting ceilings are a new refusal] → set at 16/256/4, far past any
  document and within a two-megabyte stack even in a debug build; the
  alternative was an abort (D4).
- [The `validate`-everywhere pattern (D3) is unusual chumsky] → it is the
  documented purpose of `validate` and `MapExtra::emit`; the refusal fixtures
  pin every sentence and position so a misuse is a failing test, not a
  surprise.
- [`GrammarError` grows two fields] → only `domain` constructs it; the
  `Display` change reaches `MalformedKind::Grammar.detail` text, which no
  test asserts beyond non-emptiness.
- [Sketches in this document are untested] → every one is a shape, not a
  contract; the tasks that implement them cite the fixture that proves them.

## Migration Plan

Each task in tasks.md is one commit with hooks green. Order: the dependency
swap and the parity tables (no behaviour change; the tables pass on the hand
parsers), then one layer per commit — inline, block, document, frontmatter —
each landing its chumsky implementation as the live `parse` with the hand
parser moved to a `cfg(test)` `legacy` module and the differential harness in
the same commit; then the legacy modules, harnesses and `saphyr` are
removed; then measurements and the Renovate rule.

Rollback at any point is `jj abandon` of the later commits. Because no
emitter changes, Bundles on disk are identical before, during and after;
rolling back a layer restores the hand parser for that layer and nothing
else moves. The conformance suite runs at its default count on every commit
and at `10 000` cases (`TRANSLOAD_SEEDS`) before each legacy deletion. This
change lands before the generated-input conformance change, so the seeded
suite is the only one it has to satisfy.

## Settled since the draft (2026-11-01)

- `GrammarError.at` stays. `line`/`column` are added beside it, not in place
  of it; dropping `at` is a later one-line change if nobody reads it.
- This change lands before the generated-input conformance change. The
  differential corpora and the `10 000`-case bar run on today's seeded suite
  (`TRANSLOAD_SEEDS`); when that change follows, its `parse_region` wrapper is
  a one-liner over `document::parse`.
- `frontmatter::Node` and `Entry` are `pub(crate)`. No other crate reads
  frontmatter.
- The ceilings are 16 (inline), 256 (document) and 4 (frontmatter). D4 says
  what they guard and why the inline one came down from 64 when measured;
  the values only need to sit well above any document and well below the
  stack. `dialect.md` states the inline one.
- The walker keeps reading exactly two spaces per level, as `ingest.rs` does
  today. The `dialect.md` sentence about relative indentation describes
  behaviour the code never had; matching the code wins over fixing the
  document here, and the sentence is left for a dialect change to settle.

## Open Questions

_None._
