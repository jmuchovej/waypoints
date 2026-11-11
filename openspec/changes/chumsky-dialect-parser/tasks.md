Every task is one commit with hooks green (clippy, cargo-check, nextest,
deny, treefmt, typos). The commit summary each task carries is the one it
lands with; the body states what was verified. No emitter changes in any
task: `export_is_byte_stable_and_a_fixed_point_of_ingest` and the committed
`getting-started` Bundle ingesting to the same store are the standing gate on
every commit from 2.1 onward.

## 1. Dependency

- [x] 1.1 `build(domain/dialect): take chumsky behind the syntax feature — pulldown-cmark was declared and never used` —
      add `chumsky = { version = "0.13", default-features = false, features = ["std"] }`
      to the workspace table, set `syntax = ["dep:chumsky"]`, remove
      `pulldown-cmark` from `domain`'s manifest and the workspace table;
      verify `rg pulldown` finds nothing outside the lockfile, `just deny`
      passes with no new allowance, and record the cold `cargo build --timings
-p transload` baseline (three runs, toolchain cargo) in design.md D12.
      (Bans, licenses and sources pass with no new allowance. Advisories
      fail on eleven advisories already on `main`; none names a crate this
      change adds, removes, or moves in the lockfile.)

## 2. Parity tables on the hand parsers

- [x] 2.1 `test(domain/dialect): inline parity table — every token, shortcut and escape the emitter writes reads back` —
      a table-driven test built from `TOKEN_ORDER`,
      `SHORTCUTS_WITH_INTERIOR_PARSING`, every `shortcut` arm, the
      parameterised prefixes, every mention spelling, the link, the
      `{`-opening plain run, every `ALWAYS_ESCAPED` character and the
      contextual escapes; verify it passes on the hand parser and that
      `cargo llvm-cov` shows every line of `emit_segment`, `shortcut`,
      `emit_attrs`, `escape_text`, `escape_script`, `escape_href`,
      `escape_page` and `escape_label` covered by it.
- [x] 2.2 `test(domain/dialect): block parity table — every variant, attribute key and cell escape reads back` —
      every `Content` variant themed and unthemed, every attribute key, the
      `{}` suffix, every `LINE_OPENERS` escape, both table spellings, every
      `escape_cell` rule; verify the same coverage bar over `emit_with`,
      `text_line`, `attrs`, `emit_table`, `escape_cell`, `join_sizes`,
      `join_colors`.
- [x] 2.3 `test(transload/dialect): frontmatter parity table — every reading kind, retyped id and note reads back` —
      every `Reading` variant empty and non-empty, every `plain` re-typing
      case, the icon, the `|2-` indicator, notes present and absent; verify the
      coverage bar over `emit`, `push_reading`, `push_block_scalar`, `plain`,
      `quote`.

      _Coverage measured 2026-11-02, after the port, with `cargo llvm-cov`
          0.9.1 and nixpkgs LLVM 22.1.8 (the toolchain's own LLVM), running only
          the parity tests. Every line of every named emitter function is
          reached except: `emit_attrs`' two `skip` branches and `emit_table`'s
          `unreachable!`, which no input can reach; `write_item`'s multi-line
          first entry, which the frontmatter never writes; and the
          `Reading::File` arm, since `File` has no `FeatureKind` to read it back
          with. The measurement found `coalesce`'s merge and drop paths
          unexercised, so the inline table now emits its rows uncoalesced and
          has two rows that need normalising. `push_reading`,
          `push_block_scalar` and `plain` became `frontmatter::value` and
          `yaml::write`, `write_entry`, `write_item` and `inline` in 5.1._

- [x] 2.4 `test(transload/dialect): refusal fixtures — every hand-written sentence at its line and column` —
      one fixture per `GrammarError`/`grammar(…)` site in `inline.rs`,
      `block.rs`, `ingest.rs` and `identity.rs`, asserting the sentence and,
      for now, the byte offset the hand parser reports; verify every sentence
      in the four files appears in the fixture file (a test greps the sources
      for `"unclosed`, `is not a`, `expected a` and fails on a
      sentence with no fixture).

## 3. Inline and block layers

- [x] 3.1 `refactor(domain/dialect): inline grammar on chumsky — depth-bounded runs, committed openers, refusals with line and column` —
      `inline::parser(links)` per design.md D3–D6, `GrammarError` gains
      `line`/`column`, `parse_with` builds and runs the parser; the hand
      parser moves to `inline::legacy` under `cfg(test)` with a differential
      harness over the committed Bundle's lines, the parity table, the
      conformance suite's generated stores and the malformed list; verify all
      existing `inline.rs` tests pass unchanged, the parity table passes, the
      differential reports no disagreement, and the refusal fixtures pass with
      their offsets updated to line/column where the new position is more
      specific (never less).
- [x] 3.2 `refactor(domain/dialect): block grammar on chumsky — the trailing attribute list as a forward grammar anchored at the line end` —
      `attrs_at_end`, `attributed_line`, cells and rows per D7, `block::parse`
      standalone and prefix-free; `block::legacy` and its differential; verify
      `block.rs` tests, the parity table, the differential, the refusal
      fixtures, and that `survives` cases with trailing-space and dash-only
      cells still round-trip.
- [x] 3.3 `test(domain/dialect): nesting ceiling — the deepest inline run refuses, one shallower parses` —
      fixtures at depth 16 and 17 for every nestable construct, and a
      1 MiB string of `^{` that must refuse without exhausting a 2 MiB stack
      (run on a spawned thread with that stack size); verify the refusal reads
      `inline marks nest deeper than 16 levels` at line 1, column 1 + 2·16.
      (The ceiling was 64 in the draft; see design.md D4's amendment.)

## 4. Document layer

- [x] 4.1 `refactor(transload/dialect): identity comments as a parser — the walker composes it, parse stays a wrapper` —
      `identity::marker()` returning `Marker` with `SourceKeyedIdentity` and
      the `takes a Mark id, not` refusal as emitted errors; verify
      `identity.rs` tests pass unchanged through the wrapper.
      (The refusals are `MalformedKind` values, not emitted errors; see
      design.md D8's amendment.)
- [x] 4.2 `refactor(transload/dialect): the body walker as a prefix-parameterised grammar — child regions parsed in place, spans into the file` —
      `document.rs` per D8 replacing `parse_region`/`parse_node`/`join`/
      `matching_closer`/`body_lines`; block refusals map to the real file line
      and a column; `document::legacy` and a differential over every Landmark
      file of the committed Bundle and of the conformance suite's exports,
      comparing the node trees; verify `tests/ingest.rs` and
      `tests/conformance.rs` pass unchanged, the differential agrees, and the
      refusal fixtures for `expected a `wp:mark` comment`, `a `wp:mark`
comment with no block after it`, `unclosed block, expected`, `unclosed
`::` component` and `a container's comment is never closed` pass with
      file lines. (Nesting is a Rust recursion over line views and
      `matching_closer` survives as `closer`; see design.md D8's amendment.)
- [x] 4.3 `test(transload/dialect): document refusals cite the construct's own line` —
      a Bundle whose fourth block, nested two levels down inside a blockquote,
      has an unknown mark token; verify the refusal's `Location` is that
      block's text line (not its identity comment, not its parent's) and the
      detail carries a column that accounts for the `> ` prefix; and a body
      nested 257 levels refuses with `the body nests deeper than 256 levels`.

## 5. Frontmatter

- [x] 5.1 `refactor(transload/dialect): frontmatter emit through a Node pivot — byte-identical` —
      `Node`, `Entry`, `Node::id`, and a writer; `emit` builds a `Node` and
      writes it; verify `emit`'s output is byte-equal to the pre-change
      emitter over the parity table, `frontmatter.rs` fixtures and every
      Landmark file of the committed Bundle (the old emitter kept as
      `legacy::emit` for the comparison), and that `the_shape_is_fenced_and_ordered`
      passes unchanged.
- [x] 5.2 `feat(transload/dialect): parse frontmatter as the strict subset the emitter writes — saphyr dropped, refusals name the line` —
      `map(depth)`, `seq(depth)`, scalars, flow, JSON, notes and tolerances
      per D9; interpretation transcribed from `reading`; `frontmatter::parse`
      returns a located error and `ingest` cites `Location::line`; `saphyr`
      leaves both manifests; verify every `frontmatter.rs` test passes, the
      parity table passes, `hand_mangled_advisory_content_changes_nothing` and
      `a_reference_list_ignores_its_labels` still hold, a `Node` round-trip
      property (`parse(write(n)) == n` over a generator of writer-domain
      nodes) passes at 1 000 cases, and fixtures for each refused construct
      (tag, anchor, single quote, three-space step, depth 5, multi-line flow)
      refuse naming the line. (`saphyr` leaves `[dependencies]` here and
      stays a dev-dependency for `frontmatter::legacy::parse`, the oracle the
      differential holds the subset to, until 6.2 retires it.)
- [x] 5.3 `test(transload/dialect): frontmatter tolerances — CRLF, trailing whitespace, blank lines and own-line comments parse to the same Frontmatter` —
      verify each tolerance applied to the committed Bundle's frontmatter
      yields a `Frontmatter` equal to the untouched file's. (Trailing
      whitespace is applied outside literal blocks only: inside one it is the
      text's, in YAML as here.)

## 6. Retire the oracles

- [x] 6.1 `chore(domain/dialect): retire the hand-written inline and block parsers` —
      delete `inline::legacy`, `block::legacy` and their differentials; verify
      the conformance suite passes at its default count and at 10 000 cases
      in release (`TRANSLOAD_SEEDS`) first, and that `rg legacy crates/domain`
      finds nothing. (Checked as `rg legacy crates/domain/src/syntax`: the
      word also names an icon-spelling doctest and a table fixture elsewhere
      in `domain`, neither of them a parser.)
- [x] 6.2 `chore(transload/dialect): retire the hand-written walker and frontmatter reader` —
      delete `document::legacy`, `frontmatter::legacy`, their differentials
      and the `legacy::emit` comparison; verify the same conformance bar, that
      `rg "saphyr|legacy" crates/transload Cargo.toml Cargo.lock` finds
      nothing, and record the post-port `--timings` numbers in design.md D12
      beside the baseline.

## 7. Policy and documentation

- [x] 7.1 `chore(deps): gate chumsky bumps on the dependency dashboard — every 0.x minor is a breaking release` —
      a `packageRules` entry in `.github/renovate.json5` with the description
      and `dependencyDashboardApproval: true`, in the shape of the `turso`
      rule; verify `just renovate-plan extract` lists `chumsky` under the new
      rule. (`renovate` is not in the devenv shell, so the recipe's steps ran
      under `nix shell nixpkgs#renovate` 44.181.0: the config validates, and
      extract lists `chumsky` at `0.13`, locked `0.13.0`, inherited by
      `domain` and `transload`. Extract applies no package rules, and a
      cargo lookup finds no newer stable release, so the rule has nothing to
      act on until `0.14` ships.)
- [x] 7.2 `docs(domain/dialect): dialect.md records the nesting ceiling and drops the pulldown-cmark remark` —
      one sentence under "Inlines" for the ceiling; the "Character shortcuts"
      paragraph about `pulldown-cmark` events removed; verify `openspec
validate chumsky-dialect-parser --strict` passes and `lychee` is clean.
