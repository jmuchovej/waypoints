Every task lands with hooks green (clippy, cargo-check, cargo-test, treefmt).
The acceptance gate for the whole change is unchanged behavior on host targets — `cargo build --workspace`, `cargo nextest run --workspace`, and the `store` round-trip over the committed `getting-started` Bundle all pass — plus the WebAssembly checks: `domain` and grammar-only `transload` compile for both targets with no engine, entropy, or host binding in their graphs, and a probe module built from them for `wasm32-unknown-unknown` imports nothing.
Because the per-commit hook runs with default features on the host, every WebAssembly verification runs with `-p` and `--target`, where feature unification cannot hide a mistake.

## 1. Toolchain

- [x] 1.1 Add `wasm32-wasip1` and `wasm32-unknown-unknown` to `rust-toolchain.toml` targets and let the sync task carry them into `.cargo/deny.toml`; verify `rustup target list --installed` in the devenv shell shows both and `cargo deny check` surfaces nothing the new targets introduce.

## 2. The façade's engine feature

- [x] 2.1 Add `bedrock`'s default-on `engine` feature gating the `config`, `database`, `ddl`, `durability`, `error`, `explain`, `migrate`, `pool`, `query`, `tx`, and `value` modules, their re-exports and the `SyncMode` re-export, the `turso`, `turso_core`, `tokio`, `tracing`, and `indoc` dependencies, the Android `mimalloc` dependency, and `required-features = ["engine"]` on the `crash-victim` binary; verify `cargo check -p bedrock --no-default-features` builds the façade re-exports and derives alone and `cargo nextest run -p bedrock` is unchanged.
- [x] 2.2 Set `default-features = false` on the workspace `bedrock` declaration, enable `engine` on `store` and `outpost-hold`, and leave `domain` with no features; verify `cargo build --workspace` emits no "default-features is ignored" warning and `cargo tree -p domain -e normal -i turso_core` reports the crate absent.

## 3. The grammar crate's seam

- [x] 3.1 Add `transload`'s default-on `store` feature making `bedrock` (with `engine`) and `jsonschema` optional and gating `export`, `ingest`, `IngestError::Store`, and the two reading-validation functions in `schema`, with the grammar modules and their re-exports unconditional; verify `cargo check -p transload --no-default-features` builds on the host and `cargo nextest run -p transload` is unchanged.
- [x] 3.2 Fix `layout::chart_dir` to read the Chart id through `AsRef` rather than `Display`; verify `bundle::tests::builtin_charts_keep_their_dotted_name`, the two builtin ingest tests, and `store`'s getting-started round-trip pass.

## 4. Minting and the clock, off wasm32

- [x] 4.1 Drop `v4` and `js` from the workspace `uuid`, enable `v4` on `cartograph` and `xtask`, and move `rand`, `uuid/v4`, and `chrono/now` into a `cfg(not(target_arch = "wasm32"))` target table in `domain`; verify `cargo check --workspace --all-targets` is clean and `cargo tree -p domain -e normal -i getrandom@0.4.3` on the host still names `rand` and `uuid`.
- [x] 4.2 Put `cfg(not(target_arch = "wasm32"))` on every random constructor, minting `Default`, the `touch` hook, `now_ms` and its re-exports, and the minting `From<String> for SelectOption`, and `cfg_attr` builder attributes on the id and timestamp fields of `Mark`, `Terrain`, `Landmark`, `Chart`, `Feature`, `SelectOption`, and `SelectGroup` so a wasm32 build requires them; verify `cargo check -p domain --features syntax --target wasm32-unknown-unknown` and `--target wasm32-wasip1` both compile and `cargo check -p domain --features syntax` on the host is unchanged.
- [x] 4.3 Run `cargo check -p transload --no-default-features` for both wasm32 targets and the inverse trees for `getrandom`, `wasm-bindgen`, `rand`, and `turso_core` on each; verify every check compiles and every inverse tree is empty.

## 5. The probe

- [x] 5.1 Record the pre-gating experiment in `design.md`'s Outcome: the as-is checks on `wasm32-unknown-unknown` (fail in `getrandom`), the documented recipe (compiles, binds a JavaScript host for `getRandomValues` and `now`), and the `wasip1` import section; verify each step's command, result, and attribution are written down.
- [x] 5.2 Rebuild the throwaway probe `cdylib` (outside the workspace, not committed; it needs the workspace lockfile and the workspace's `specta-macros` patch) against the gated data model for both wasm32 targets with no configuration; verify the `unknown-unknown` module's import section is empty and the `wasip1` module imports only what the standard library's startup and standard error need, and record both sections verbatim in the Outcome.

## 6. Gate

- [x] 6.1 Run `cargo build --workspace`, `cargo nextest run --workspace`, `cargo clippy --workspace --all-targets`, `cargo deny check`, and all four WebAssembly checks; verify the host suite passes in full and the WebAssembly checks are clean, and record the measured check times in `design.md`.
      (817 of 817 host tests pass; clippy's remaining warnings are pre-existing and in files this change does not touch; `cargo deny` passes licenses, bans, and sources and fails only on the twelve pre-existing advisories recorded in the Outcome.)
- [x] 6.2 Verify `openspec validate grammar-only-builds --strict` passes and the Outcome section is complete.
