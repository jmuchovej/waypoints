Every task lands with hooks green (clippy, cargo-check, cargo-test, treefmt).
The four conformance properties keep their names and meaning throughout; a
rewrite task is done when the same property passes at the same case count on
the new engine.

## 1. Toolchain

- [ ] 1.1 Settle the fuzz toolchain: try `cargo fuzz build -s none` on the
      pinned stable; if it builds, the `fuzz` environment provides only
      `cargo-fuzz`, otherwise it adds a pinned nightly. Verify
      `mise use`/devenv selection of `fuzz` makes `cargo fuzz --version` print,
      and record the outcome in `design.md` D6.
- [ ] 1.2 Add `arbitrary`, `proptest`, `proptest-arbitrary-interop`, and
      `libfuzzer-sys` to the workspace dependency table; verify
      `cargo deny check licenses bans sources` passes.

## 2. One generator

- [ ] 2.1 Add `domain::testing` behind a `testing` feature with `Arbitrary` for
      `TextMarkKind`, `Color`, `Size`, `MathEngine`, `BlockTheme` (derived) and
      `InlineMark`, `TextSegment`, `RichText`, `Content` (hand-written,
      domain-valid, `RichText` coalesced); verify a unit test over 10 000
      arbitrary values finds every `TextMarkKind` and every `Content` variant
      and no `InlineMark` whose data the emitter cannot spell.
- [ ] 2.2 Add `proptest!` round trips in `domain`'s syntax tests —
      `inline::parse(emit(v)) == v` and `block::parse(emit(v)) == v` over
      `arb::<RichText>()` and `arb::<Content>()` at 1 000 cases — with
      `proptest-regressions/` committed; verify they pass and that
      `PROPTEST_CASES=100000` passes.

## 3. Conformance suite on `proptest`

- [ ] 3.1 Add store strategies to `transload/tests/common`: `chart()`,
      `landmark_for(&Chart)` (readings fitting each feature kind, titles that
      sometimes repeat or are empty), `store(&Chart)` (nested content trees,
      a shared Mark, a nested Landmark, one edge of every non-structural kind,
      mentions of Landmarks, blocks, and missing targets); verify a smoke test
      generates 64 stores that `export` without warnings.
- [ ] 3.2 Rewrite the four properties in `tests/conformance.rs` as `proptest!`
      tests over `store()` with a per-case current-thread runtime,
      `ProptestConfig { cases: 64 }`, and persisted regressions; delete `Gen`,
      `TRANSLOAD_SEEDS`, and the `rand` dev-dependency in the same commit;
      verify all four pass at 64 and at `PROPTEST_CASES=10000` in release.
- [ ] 3.3 Prove shrinking: temporarily reintroduce the trailing-backslash cell
      bug in `block::split_cells` (or its successor) and verify the reported
      minimal store is a single Landmark with a one-cell table, then revert;
      record the shrunk reproducer shape in this task.

## 4. Fuzz targets

- [ ] 4.1 Create `crates/transload/fuzz/` (excluded from the workspace) with
      `inline_total`, `block_total`, `region_total`, `inline_roundtrip`,
      `block_roundtrip`; add a `fuzzing` feature on `transload` exposing the
      region parser; verify `cargo check --manifest-path
crates/transload/fuzz/Cargo.toml` passes on the stable toolchain.
- [ ] 4.2 Add `just fuzz-seed`, writing every comment-stripped block line of the
      committed `getting-started` Bundle plus 500 emitter outputs per layer
      into `fuzz/corpus/<target>/`; gitignore `corpus/` and `artifacts/`;
      verify each corpus directory is non-empty after the recipe.
- [ ] 4.3 Add `just fuzz <target> [seconds]` selecting the `fuzz` environment
      and running the target against its corpus; verify every target runs for
      60 seconds without a crash, and record any crash found as a minimized
      unit test in the owning crate before this task closes.

## 5. Hooks and documentation

- [ ] 5.1 Add a `fuzz-check` step to `.config/hk.pkl` that type-checks the fuzz
      crate on the stable toolchain, scoped to changes under `crates/domain`,
      `crates/transload`, and the fuzz crate; verify a deliberate signature
      change to `inline::parse` fails the hook, then revert.
- [ ] 5.2 Update `dialect.md` (the conformance-suite note: `PROPTEST_CASES`, the
      fuzz recipes) and record the measured first-run cost of the fuzz check
      in `design.md`; verify `openspec validate proptest-fuzz-conformance
--strict` passes.
