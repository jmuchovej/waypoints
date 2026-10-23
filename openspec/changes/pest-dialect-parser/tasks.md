Every task lands with hooks green (clippy, cargo-check, cargo-test, treefmt).
The acceptance gate for the port is unchanged tests plus the conformance suite
at `TRANSLOAD_SEEDS=10000`; a dialect discrepancy found on the way is fixed so
the old parser, the grammar, and `dialect.md` agree, never by changing what the
emitter writes.

## 1. Dependency and grammar skeleton

- [ ] 1.1 Add `pest` and `pest_derive` to the workspace and to `domain` behind
      the `syntax` feature; verify `cargo deny check licenses bans sources`
      passes and `cargo tree -p transload -i pest` shows the one path.
- [ ] 1.2 Add `crates/domain/src/syntax/grammar/inline.pest` and `block.pest`
      with the rule names the error map will use, derived parsers in a private
      `syntax::grammar` module, and the treefmt exclusion; verify
      `cargo build -p domain --features syntax` compiles both grammars.
- [ ] 1.3 Record the `syntax`-feature compile-time delta (`cargo build -p domain
--features syntax --timings`, before and after) in this task's
      description once measured.

## 2. Inline grammar and walker

- [ ] 2.1 Move the current `inline::Parser` to a private `syntax::legacy::inline`
      module unchanged; verify every `inline.rs` test still passes against it.
- [ ] 2.2 Write `inline.pest`: escapes, mention (`page`, `#block`, `|label`),
      bracketed run with `(href)` or `{tokens}` suffix, verbatim code and math
      interiors, braced scripts, paired shortcuts, literal text; verify each
      rule with a `#[test]` per construct using `pest`'s `parses_to!`.
- [ ] 2.3 Implement `inline::parse_with` as a walker over the pair tree that
      builds `TextSegment`s, applies marks outward, resolves mention targets
      through `Links`, and runs `coalesce`; verify every existing `inline.rs`
      test passes unchanged against the new parser.
- [ ] 2.4 Add the differential test: the conformance generator's `rich()` output
      through `emit_with` is parsed by both parsers and compared, accepted or
      refused alike, over `TRANSLOAD_SEEDS` seeds; verify 10 000 seeds agree.

## 3. Block grammar and walker

- [ ] 3.1 Write `block.pest`: attribute list (theme token, `key="value"` pairs,
      escaped closing brace), ATX headings, bullets and enums, blockquote
      opener, divider, adaptive code fence via `PUSH`/`PEEK`/`POP`, math fence,
      `::name{…}` component opener and closer, GFM rows with `\|` cells and
      the delimiter row; verify each with `parses_to!` tests.
- [ ] 3.2 Implement `block::parse_with` over the pair tree, handing each text
      position to `inline::parse_with`; delete `split_attrs`, `parse_attrs`,
      `split_tokens`, `split_cells`, `is_delimiter_row`, and the fence readers;
      verify every existing `block.rs` test passes unchanged.
- [ ] 3.3 Extend the differential test to `block::emit_with` output over
      generated `Content`, both parsers compared; verify 10 000 seeds agree.
- [ ] 3.4 Run `cargo nextest run -p transload` and the conformance suite at
      `TRANSLOAD_SEEDS=10000`, and the `store` round-trip test over the
      committed `getting-started` Bundle; verify all pass with the new parsers
      live.

## 4. Refusals

- [ ] 4.1 Build the rule-name → dialect-vocabulary map with
      `Error::renamed_rules`, extend `GrammarError` with `line` and `column`,
      and keep the hand-written sentences the grammar cannot express (the
      bracketed-run suffix rule, unknown block attributes); verify the
      refusal assertions in `inline.rs`, `block.rs`, and
      `transload/tests/ingest.rs` pass unchanged and that no refusal `detail`
      contains a raw rule identifier (a test over every grammar rule name).
- [ ] 4.2 Add the totality test: arbitrary strings from the conformance
      alphabet, including unmatched openers, lone trailing backslashes, and
      deep nesting, parse or refuse without panicking; verify it over
      `TRANSLOAD_SEEDS` seeds.

## 5. Parity and cleanup

- [ ] 5.1 Add the emitter-parity test: for every `TextMarkKind` and every
      data-bearing token value the emitter writes, a one-segment `RichText`
      renders and parses back equal; verify it passes and that removing any
      row from `TOKEN_ORDER` or `PAIRED` makes it fail.
- [ ] 5.2 Delete `syntax::legacy` and the differential tests; verify the
      workspace builds, `cargo nextest run --workspace` passes, and clippy is
      clean for `domain` and `transload`.

## 6. Documentation

- [ ] 6.1 Add the "Executable grammar" section to `dialect.md` naming both
      files and their precedence, and cut prose that restates a production to
      its rationale; verify the document still covers every policy the grammar
      cannot carry (canonical form, normalization, mention naming) by reading
      it against the spec's requirements.
- [ ] 6.2 Record the decisions and the measured compile-time delta in this
      change's `design.md`; verify `openspec validate pest-dialect-parser
--strict` passes.
