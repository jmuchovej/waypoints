## Context

See proposal.md — Why. The constraints:

- `tests/conformance.rs` holds a 700-line generator (`Gen`) plus four
  properties. Each case opens an in-memory turso database
  (`common::open()`), writes a generated store, exports, ingests into a second
  database, and compares. Cases are therefore milliseconds, not microseconds;
  the suite runs 64 by default and 10 000 in about a minute in release.
- The generator encodes relational validity the type system does not: a
  `Reading` must fit its feature's `FeatureKind`; a `Select` glyph must exist
  in the feature; a mention targets a Mark the store holds; two Landmarks may
  share a title; a Content Mark may be shared into two parents.
- Mark data is `Option<Json>`; which values are legal for which
  `TextMarkKind` is a domain convention (`bg-{color}`, `text-{size}`), not a
  type.
- The workspace toolchain is nix-pinned stable 1.98.1 via
  `rust-toolchain.toml`; there is no `rustup`. Developer tools arrive through
  mise environments projected from nix (`cargo-nextest` via `test`), which the
  `toolchain/tool-projection` spec says compose additively.
- Per-commit hooks (`.config/hk.pkl`) run clippy, check, nextest over a
  computed filterset, treefmt, typos. `cargo nextest run --workspace` builds
  every workspace member.
- `libfuzzer-sys` compiles libFuzzer's C++ with `cc`; building a fuzz target
  for running needs `-Zsanitizer`/sancov flags, which is nightly territory.
  Type-checking the crate does not.

## Goals / Non-Goals

**Goals:**

- Same four properties, same acceptance; failures shrink and persist.
- One `Arbitrary` generator per syntax type, reused by both frameworks.
- Fuzz targets for inline, block, and document-region parsing, seeded from
  the committed Bundle, runnable with one command, type-checked on every
  commit.

**Non-Goals:**

- Running fuzzers in CI; a scheduled job is a follow-up once targets exist.
- Generators for the sync protocol or bedrock documents; the module is open
  to them but this change seeds only the dialect.
- Making the fuzz crate a workspace member — see D4.

## Decisions

### D1 — `arbitrary::Arbitrary` in `domain` is the single generator; `proptest` adapts it (2026-10-23)

The syntax types get `Arbitrary` impls in `domain::testing` behind a `testing`
feature. Derives where every value is valid (`TextMarkKind`, `Color`, `Size`,
`MathEngine`, `BlockTheme`); hand-written where data is constrained
(`InlineMark`, so a `Highlight` carries a colour name or nothing, a `Mention`
an id, a `Link` an href; `Content::Code`, so the language is a token;
`RichText`, built from segments then `coalesce`d so the generated value is
already canonical). `proptest` reaches these through
`proptest_arbitrary_interop::arb::<T>()`, which drives an `Arbitrary` impl
from `proptest`'s byte source and so shrinks it.

_Rejected:_ `proptest::Arbitrary` as the primary — fuzz targets consume
`arbitrary::Arbitrary` (libFuzzer bytes in, structured value out) and the
interop crate only goes one way. _Rejected:_ two independent impls — that is
the duplication the proposal names.

### D2 — Relational generation stays in `proptest` strategies (2026-10-23)

A store is relational: feature kinds constrain readings, glyph ids must
exist, mention targets must be Marks in the store, shared Marks and nested
Landmarks are edges between generated ids. `Arbitrary` has no way to say "a
value consistent with that other value"; `proptest`'s `prop_flat_map` does.
`transload/tests/common` gains `chart()`, `landmark_for(&Chart)`,
`store(&Chart)` strategies that compose `arb::<RichText>()` and
`arb::<Content>()` with the relational layer, replacing `Gen::reading`,
`Gen::content`, `tree`, and `generate`.

Shrinking a relational store is where `proptest` earns its keep: a failing
case shrinks the tree, then the readings, then each rich text, and the
reproducer that persists is the seed of the shrunk value, not the original.

### D3 — Async inside `proptest!` (2026-10-23)

Each case builds a current-thread tokio runtime and `block_on`s the
export–ingest cycle. `proptest` is synchronous and cases are already
database-bound, so the runtime cost is noise.

_Rejected:_ `tokio::test` + manual loop (the current shape) — it is what
loses shrinking. _Rejected:_ a shared runtime across cases — `proptest!`
closures are `Fn`, and the in-memory database per case must not outlive it.

### D4 — The fuzz crate lives at `crates/transload/fuzz` and outside the workspace (2026-10-23)

`cargo fuzz` looks for `fuzz/` under the crate it is run from; `transload`
depends on `domain`, so one fuzz crate reaches both layers and the document
walker. It is listed in the root `Cargo.toml` `exclude`, so
`cargo nextest run --workspace` does not compile libFuzzer's C++ on every
run; the hook step type-checks it explicitly (`cargo check --manifest-path
crates/transload/fuzz/Cargo.toml`), which does compile libFuzzer once and is
then cached.

Targets:

| target             | input                 | property                                 |
| ------------------ | --------------------- | ---------------------------------------- |
| `inline_total`     | bytes                 | `inline::parse` returns or refuses       |
| `block_total`      | bytes                 | `block::parse` returns or refuses        |
| `region_total`     | bytes                 | `ingest` region parse returns or refuses |
| `inline_roundtrip` | `Arbitrary<RichText>` | `parse(emit(v)) == v`                    |
| `block_roundtrip`  | `Arbitrary<Content>`  | `parse(emit(v)) == v`                    |

The region parser is `pub(crate)` today; it gains a `pub` wrapper behind a
`fuzzing` feature on `transload` so the target can reach it without widening
the crate's real surface.

### D5 — Seed corpora from the committed Bundle (2026-10-23)

A `just fuzz-seed` recipe writes every block line of
`crates/store/src/collections/getting-started/` (comments stripped, one line
per file) into `fuzz/corpus/<target>/`, plus a few hundred emitter outputs
from the `Arbitrary` generators. libFuzzer then mutates valid inputs rather
than rediscovering `[[` from random bytes. `corpus/` and `artifacts/` are
gitignored; a crash is minimized with `cargo fuzz tmin` and lands as a unit
test in the owning crate, which is the committed form.

_Rejected:_ committing corpora — they grow without bound and encode nothing a
test does not.

### D6 — Nightly via a `fuzz` environment; stable keeps the workspace (2026-10-23)

`cargo fuzz run` needs a nightly for sanitizer and coverage flags. The
`fuzz` mise environment, projected from nix like `test`, provides a pinned
nightly and `cargo-fuzz`; the `just fuzz <target>` recipe selects it. The
workspace toolchain and every hook stay on the pinned stable; only the fuzz
check runs there, and it needs no nightly.

Task 1.1 first tries `cargo fuzz build -s none` on the stable toolchain; if
that builds, the environment provides only `cargo-fuzz` and the nightly is
dropped. The decision is the same either way — fuzz tooling is an
environment, not a workspace toolchain change.

### D7 — `PROPTEST_CASES`, default 64 (2026-10-23)

`ProptestConfig { cases: 64, .. }` in the suite, overridden by `proptest`'s
own `PROPTEST_CASES` variable. `TRANSLOAD_SEEDS` is removed rather than
aliased; the `dialect.md` note updates. Regressions persist to
`crates/transload/proptest-regressions/` (and `crates/domain/…` for the
inline/block properties), committed.

### D8 — Fast properties in `domain`, slow ones in `transload` (2026-10-23)

Inline and block round trips need no database and run at ~10⁴ cases per
second; they live as `proptest!` tests in `domain`'s `syntax` tests with a
higher default count (1 000). The store-level properties stay in `transload`
at 64. The bugs the hand-rolled suite found were all in the dialect layer;
this is where cheap cases pay.

## Risks / Trade-offs

- [`proptest-arbitrary-interop` shrinks by shrinking the byte source, which
  can shrink structured values less cleanly than native strategies] →
  acceptable for the leaf types; the relational layer (D2) is native
  `proptest` and shrinks structurally. If leaf shrinking proves poor, the
  interop can be swapped for native strategies per type without touching
  callers.
- [Two frameworks, two vocabularies] → one generator module (D1) and one
  recipe family (`just fuzz*`); the proposal's Impact lists every new crate.
- [libFuzzer's C++ build adds to the first hook run] → cached thereafter;
  measured in task 3.1.
- [A nightly in the environment] → scoped to `fuzz`; nothing in the workspace
  reads it. D6's stable path removes it if it works.
- [Regressions files accrete] → each line is a bug that was real; they are
  reviewed like tests and pruned when the property they belonged to is
  deleted.
- [The in-memory database per case bounds throughput] → unchanged from today;
  the fast properties move to `domain` (D8) where the parser bugs were.

## Migration Plan

Each task is a commit with hooks green. The `Arbitrary` module lands first
(no behaviour change), then the `domain` properties (new tests only), then the
conformance rewrite (same properties, new engine; the old `Gen` is deleted in
the same commit so there is one generator), then the fuzz crate, environment,
and hooks. Rollback at any point is `jj abandon` of the later commits; the old
suite remains the live one until its rewrite lands.

## Open Questions

- Whether `cargo fuzz build -s none` runs on the pinned stable (D6). Settled
  by task 1.1; either answer leaves the targets, recipes, and hook unchanged.
