Every task lands with hooks green (clippy, cargo-check, cargo-test, treefmt).
The acceptance gate for the whole change is unchanged behavior under default features — `cargo build --workspace`, `cargo nextest run --workspace`, and the `store` round-trip over the committed `getting-started` Bundle all pass as before — plus the two `wasm32-wasip1` checks that prove the engine is gone from the grammar crates.
The `wasm32-unknown-unknown` steps are an experiment: their verification is that the outcome is recorded, not that they pass.
Because the per-commit hook runs with default features, every verification below runs with `-p` and `--no-default-features`, where feature unification cannot hide a mistake.

## 1. Toolchain

- [ ] 1.1 Add `wasm32-wasip1` and `wasm32-unknown-unknown` to `rust-toolchain.toml` targets and let the sync task carry them into `.cargo/deny.toml`; verify `rustup target list --installed` in the devenv shell shows both and `cargo deny check` passes, triaging any new advisory or duplicate with an `ignore` and reason.

## 2. The façade's engine feature

- [ ] 2.1 Add `bedrock`'s default-on `engine` feature gating the `config`, `database`, `ddl`, `durability`, `error`, `explain`, `migrate`, `pool`, `query`, `tx`, and `value` modules, their re-exports and the `SyncMode` re-export, the `turso`, `turso_core`, `tokio`, `tracing`, and `indoc` dependencies, the Android `mimalloc` dependency, and `required-features = ["engine"]` on the `crash-victim` binary; verify `cargo check -p bedrock --no-default-features` builds the façade re-exports and derives alone and `cargo nextest run -p bedrock` is unchanged.
- [ ] 2.2 Set `default-features = false` on the workspace `bedrock` declaration, enable `engine` on `store` and `outpost-hold`, and leave `domain` with no features, in one commit; verify `cargo build --workspace` emits no "default-features is ignored" warning, `cargo nextest run --workspace` passes, and `cargo tree -p domain -e normal -i turso_core` reports the crate absent.

## 3. The data model's own manifest

- [ ] 3.1 Enable chrono's `now` feature on `domain`'s chrono dependency; verify `cargo check -p domain --no-default-features --features syntax` builds on the host with no other member in the graph.
- [ ] 3.2 Run `cargo check -p domain --no-default-features --features syntax --target wasm32-wasip1`; verify it compiles, or record which transitive dependency fails and resolve it per the design's fallback (gate it where it belongs, or report it) before continuing.

## 4. The grammar crate's seam

- [ ] 4.1 Add `transload`'s default-on `store` feature making `bedrock` (with `engine`) and `jsonschema` optional and gating `export`, `ingest`, `IngestError::Store`, and the two reading-validation functions in `schema`, with the grammar modules and their re-exports unconditional; verify `cargo check -p transload --no-default-features` builds on the host and `cargo nextest run -p transload` is unchanged.
- [ ] 4.2 Run `cargo check -p transload --no-default-features --target wasm32-wasip1` and `cargo tree -p transload --no-default-features -e normal -i turso_core`; verify the check compiles and the tree is empty, resolving any failing transitive dependency by gating it behind `store`.

## 5. The `wasm32-unknown-unknown` experiment

- [ ] 5.1 Run `cargo check -p domain --no-default-features --features syntax --target wasm32-unknown-unknown` and the same for `transload`, as is; verify the outcome is recorded in `design.md`'s Outcome section — compiled, or the first failing dependency and its error.
- [ ] 5.2 If 5.1 failed, add `--cfg getrandom_backend="wasm_js"` for the `wasm32-unknown-unknown` target to `.cargo/config.toml` and rerun both checks; verify the outcome is recorded — compiled under the cfg, or still failing and on which dependency — and that the cfg line is kept only if it was needed and is annotated as experiment configuration.
- [ ] 5.3 Build a throwaway probe `cdylib` (outside the workspace, not committed) depending on `transload` with default features off and exporting one function that serializes a `Content` value, for `wasm32-unknown-unknown` under whatever configuration 5.1 or 5.2 found works, and list its import section with `wasm-tools print` or `wasm-objdump -x`; verify the import section is recorded verbatim in the Outcome section, with each imported module attributed to the crate that pulls it in via `cargo tree -i`.

## 6. Gate

- [ ] 6.1 Run `cargo build --workspace`, `cargo nextest run --workspace`, the `store` round-trip test over `getting-started`, `cargo deny check`, and both `wasip1` checks; verify all pass, and record in `design.md` any transitive dependency that had to be gated and the measured time of the two `wasip1` checks.
- [ ] 6.2 Verify `openspec validate grammar-only-builds --strict` passes and the Outcome section is complete.
