## Why

The `transload` conformance suite proves the Bundle's round trip over
generated stores, but its generator is a hand-rolled seeded RNG
(`tests/conformance.rs`, `Gen`): a failure hands back an entire random store
and a seed number, so finding the one bad table cell in seed 859 meant diffing
two debug dumps by script, and there is no shrinking, no persisted
reproducer, and no coverage guidance. Four dialect bugs survived 64 seeds and
were only found by hand-raising the count to 2 000 and 10 000. The generator
also encodes the domain's invariants (which `Reading` fits which
`FeatureKind`, which mark data values are legal) a second time, in test code
nobody else can reuse. Property-testing and fuzzing frameworks exist to do
exactly this work — minimize failures, remember them, and steer inputs toward
unexplored branches — and the parser is the kind of code they find bugs in.

## What Changes

- The hand-rolled `Gen` is replaced by `proptest` strategies. The four
  conformance properties keep their names and meaning; what changes is that a
  failing case shrinks to a minimal store, the reproducer is persisted in a
  committed regressions file that re-runs on every test run, and the case
  count follows `PROPTEST_CASES` (default 64, so an ordinary run stays fast)
  instead of the bespoke `TRANSLOAD_SEEDS`.
- One generator of domain-valid values serves both frameworks: `domain` gains
  a feature-gated `testing` module with `arbitrary::Arbitrary` implementations
  for the syntax-bearing types (`RichText`, `TextSegment`, `InlineMark`,
  `TextMarkKind`, `Content`, `BlockTheme`, `Color`, `Size`, `MathEngine`)
  that produce only values the domain admits (a mention's data is an id, a
  highlight's data is a colour name, a code block's language is a token). The
  `proptest` strategies compose these through `proptest-arbitrary-interop`;
  the relational layer — a Chart, Landmarks whose readings fit its features,
  edges between existing Marks — stays as `proptest` strategies in `transload`'s
  test support, because that is what `prop_flat_map` is for.
- Fast, database-free round-trip properties land in `domain` itself: inline
  `emit → parse` and block `emit → parse` over generated values, so the dialect
  is checked where it lives, at thousands of cases per second, not only
  through a store.
- Coverage-guided fuzzing with `cargo-fuzz` (libFuzzer) under
  `crates/transload/fuzz/`: totality targets that feed arbitrary bytes to
  inline parse, block parse, and the document-region parser and require no
  panic; round-trip targets that feed `Arbitrary` values to `emit → parse` and
  require equality. Each target is seeded from a corpus drawn from the
  committed `getting-started` Bundle and the emitter's output, so the fuzzer
  starts from valid inputs and mutates outward. A minimized crash becomes a
  unit test in the crate that owns the bug; corpora and artifacts are not
  committed.
- Fuzz targets are compiled in the per-commit hooks so they cannot rot;
  running them is a `just` recipe, not a hook. The nightly toolchain and
  `cargo-fuzz` arrive through a dedicated `fuzz` environment, in the same way
  `cargo-nextest` arrives through `test`, so the workspace toolchain stays the
  pinned stable.

## Capabilities

### New Capabilities

- `toolchain/conformance-testing`: how generated-input tests behave in this
  repository — failures shrink and persist as committed reproducers, every
  parser entry point has a coverage-guided fuzz target seeded from real
  Bundles, fuzz targets always compile, and the case count is one environment
  variable away from a deep run.

### Modified Capabilities

_None._ The `transload` requirements the suite checks (round-trip equality,
byte stability, the 1980 test, schema validity) do not change; this change is
about how they are checked.

## Non-goals

- Changing any property the suite asserts, or the dialect it exercises.
- Continuous fuzzing in CI (a scheduled job, ClusterFuzzLite, OSS-Fuzz).
  Targets must build in CI; running them stays a developer action until there
  is a reason to spend the minutes. Noted as a follow-up.
- Fuzzing the legacy `importer` crate, the sync protocol, or bedrock. The
  harness is general, but this change seeds only the dialect and Bundle
  parsers.
- Replacing `rstest` or the fixture-style unit tests. Property tests sit next
  to them, not in place of them.

## Impact

- `crates/domain`: a `testing` feature pulling `arbitrary` (derive and
  hand-written impls); no effect on default builds.
- `crates/transload`: `proptest` and `proptest-arbitrary-interop` as
  dev-dependencies; `tests/conformance.rs` rewritten around strategies;
  `tests/common` gains the store strategies; a committed
  `proptest-regressions/` directory; `rand` leaves the dev-dependencies.
- `crates/transload/fuzz/`: a cargo-fuzz crate outside the workspace members
  (`libfuzzer-sys`, `arbitrary`), with five targets and a corpus-seeding
  helper; `corpus/` and `artifacts/` ignored.
- Toolchain: a `fuzz` environment providing a nightly toolchain and
  `cargo-fuzz`; a `just fuzz` recipe family; a `fuzz-check` hook step that
  type-checks the fuzz crate on the stable toolchain. `cargo deny`: all new
  crates are MIT/Apache-2.0.
- Documentation: `dialect.md`'s pointer to the conformance suite and the
  `TRANSLOAD_SEEDS` note update to `PROPTEST_CASES` and the fuzz recipes.
