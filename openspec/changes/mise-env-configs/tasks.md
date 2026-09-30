## 1. Early-init settings

- [x] 1.1 Commit `.config/miserc.toml` carrying only the discovery keys mise honours there (`auto_env`, `env_conf_d`); verify behaviourally against a control that the file is read — an `env = [...]` entry loads the named config, and removing the file stops it
- [x] 1.2 Move `[settings]` — `experimental`, `lockfile`, `idiomatic_version_file_enable_tools`, `locked`, `locked_verify_provenance`, `minimum_release_age` — into a hand-maintained `.config/mise/conf.d/base.toml` carrying `#:schema`, and drop it from the projection; verify each reads back in force, `idiomatic_version_file_enable_tools` behaviourally via `mise ls --current` resolving `rust` from `rust-toolchain.toml`, and verify treefmt lints the new file rather than skipping it as it must skip generated `mise.toml`. Omit `safe`: mise refuses it outside a global config
- [x] 1.3 Confirm `locked = true` does not break a cold `mise install` — verified in a scratch clone with an isolated `MISE_DATA_DIR`: `mise install --dry-run` resolves all 33 tools and exits 0. Note that `.config/mise/mise.lock` must ship alongside `.config/mise.lock`: `rust` reaches mise through `rust-toolchain.toml` rather than `[tools]`, its lock entry lands in the conf.d lockfile, and omitting it fails the install with `rust@stable is not in the lockfile`. A real provenance-checked install was not exercised — no GitHub token is available and the anonymous quota is spent; that is 7.6's job

## 2. Declare environments and dependencies

- [x] 2.1 Add a `waypoints.mise.environments` attrset declaring `xtask`, `test`, `format`, `build-desktop`, `build-mobile`, and `development`, each with a `composes` list defaulting to `[ ]`; verify `nix flake check` passes with all of them empty
- [x] 2.2 Add `envs` to `tool-type` as a `listOf (enum <declared environment names>)` defaulting to `[ ]`. Note that nix forces an option's type only when the value is read, so while nothing consumes `envs` a bogus name evaluates clean — observed. The enum becomes protective once 3.1 computes membership from it; the negative case is verified there
- [x] 2.3 Add `depends` to `tool-type` as a list of module-level tool names, projected into the mise entry as mise's displayed names with any `core:` prefix stripped; verify `gem:rufo` renders `depends = [ "ruby" ]` while the config key stays `core:ruby`
- [x] 2.4 Assert that every `depends` entry names a declared tool; verified against a misspelled dependency, which fails with `waypoints.mise.tools.rufo.depends: no projected tool named ``rubby``.`
- [x] 2.5 Declare the real dependencies — `gem:fastlane` and `gem:rufo` on `ruby`, `npm:oxfmt` and `npm:wrangler` on `node` — and verify `mise ls --current` still resolves every tool with the base config unchanged otherwise

## 3. Generate the environment files

- [x] 3.1 Compute each environment's membership as the transitive closure of its declared members over `depends`, and verify by unit-checking that an environment containing only `gem:fastlane` yields both `fastlane` and `ruby`
- [x] 3.2 Expand `composes` so an environment's membership includes every member of the environments it composes, transitively; verify `development` declared as composing the other five yields their union
- [x] 3.3 Derive `files` so the base entry is joined by `.config/mise.<env>.toml` and a seeded `.config/mise.<env>.lock` for each environment with at least one member; with all tools still defaulting to base, verify no environment file is emitted
- [x] 3.4 Keep `settings`, `min_version`, and `[env]` (including the `_.file` pointer at `.env`) in the base only — environment files carry `[tools]` and nothing else; verify by inspecting a generated environment file once section 5 populates one
- [ ] 3.5 **Moved to group 6.** Deleting `[vars].WAYPOINTS_FORMATTERS` breaks `cargo xtask workflows` — `utils::mise::var` errors when the key is absent — and `just check-generated` runs that very command, so the projection edit cannot be verified before 6.1 removes the reader. Land both together
- [ ] 3.7 Ensure an environment file is removed when its environment empties: `devenv:files:cleanup` was observed leaving `.config/mise.<env>.toml` and its lockfile behind after the environment stopped being generated, and mise keeps loading a stale file. `check-generated` cannot catch it — it diffs only the files devenv currently declares. Decide between a cleanup step and a check over `.config/mise.*.toml` that no longer correspond to a declared environment
- [x] 3.6 Add the generated `.config/mise.*.toml` files to `.config/treefmt.toml`'s excludes and narrow the `[[schemas]]` glob in `.config/tombi.toml` to the hand-edited configs only; verify `just check-generated` converges across two consecutive shell entries. Schemas serve LSP-assisted edits — hand or agent — so generated files do not want one; a malformed tool table is caught by mise parsing it or failing loudly, which is the touchpoint that motivates generating it at all. Without the exclusion tombi indents `[tools]` entries while devenv writes them flush-left, and the check never converges

## 4. Lock verification vs regeneration

- [ ] 4.1 Split `nix/devenv/bin/sync-mise` into a verify-only script (no network, no writes) and a `lock` script that regenerates; verify entering the shell performs no `mise lock` and leaves `.config/mise.lock` byte-identical
- [ ] 4.2 Change the `mise:lock` devenv task to run the verify-only script and drop `.config/mise.lock` from its `execIfModified`, so the task no longer watches a file it writes; verify two consecutive shell entries produce no diff
- [ ] 4.3 Add a `just mise-lock` recipe that regenerates every environment's lockfile in one authenticated run, mapping `GH_TOKEN` to `GITHUB_TOKEN` as the current script does; verify it fails loudly with a stated reason when no token is present rather than silently writing a short lockfile
- [ ] 4.4 Replace the resolution-only assertion with a platform-coverage check: per environment, compare declared tools × target platforms against the lockfile's entries, with a named exemption list for backends that produce none (`gem:`, `npm:`, `rust`); verify it passes on the current full lockfile
- [ ] 4.5 Truncate a copy of `.config/mise.lock` and verify the coverage check fails and names the missing tool/platform pairs — the check is untrustworthy until its negative case is observed
- [ ] 4.6 Update `devenv_update.rs` so the PR body and commit file list enumerate every mise config and lockfile rather than the two literal paths, and verify the regenerated `devenv-update.yaml` reflects it

## 5. Partition the tool set

- [ ] 5.1 Read each generated job in `.github/workflows/` and record which tools it actually invokes; produce that job→tool mapping as the input to the partition rather than assigning `envs` by intuition
- [ ] 5.2 Assign `envs` on every tool in `devenv.nix` from that mapping, replacing each `formatter = true` with `envs = [ "format" ]`; verify the base retains only tools every environment needs
- [ ] 5.3 Declare `development.composes` as the other five environments; verify its generated file is their union and that no tool is listed in it that is absent from every other environment
- [ ] 5.4 Delete the `formatter` option from `tool-type` and the `formatter-specs` binding from `nix/devenv/mise.nix`; verify `nix flake check` and shell entry both succeed
- [ ] 5.5 Regenerate the config files and verify each `.config/mise.<env>.toml` contains exactly its closure, with OS-restricted tools keeping their inline `os = [ ... ]` table form
- [ ] 5.6 Run `just mise-lock` and verify each environment's lockfile passes the 4.4 coverage check
- [ ] 5.7 For each environment, run `MISE_ENV=<env> mise ls --current` in a scratch clone and verify the resolved set is base plus that environment's closure — confirming additivity in practice, not just in principle
- [ ] 5.8 Verify composition end to end: `MISE_ENV=xtask,test mise ls --current` resolves the union of both, matching the two-environment result measured during design

## 6. Retire the deny-list

- [ ] 6.1 Delete `formatters()` and `FORMATTERS_VAR` from `tooling/xtask/src/utils/mise.rs`, keeping `version()`; verify `cargo check` passes
- [ ] 6.2 Delete `without_formatters()` from `_jobs.rs` and `formatters` from `WorkflowContext`; verify `cargo check` passes and no `MISE_DISABLE_TOOLS` reference remains under `tooling/`
- [ ] 6.3 Add a Rust enum whose variants match the nix environment enum, and give each job an explicit list of them so a typo cannot be written; verify every generated job carries exactly one `MISE_ENV` whose value is a comma-joined list of valid names
- [ ] 6.4 Regenerate all workflows and verify no `MISE_DISABLE_TOOLS` line survives, no job names `development`, and the stale `aqua:mic92/nixfmt-rs` reference is gone from the tree
- [ ] 6.5 Run `just check-generated` and verify it reports the committed tree in lockstep with both devenv's and xtask's outputs

## 7. Verification

- [ ] 7.1 Run the full per-commit hook set (clippy, cargo-check, cargo-test, treefmt) and verify it is green
- [ ] 7.2 Push a branch and verify one job per environment — an `xtask` job, a `test` job, a `build-desktop` job, and a `format` job — installs only its environment's closure, observed in the mise-action log
- [ ] 7.3 Verify a job whose environment omits a tool it needs fails on a missing binary rather than silently falling back, by temporarily removing one tool from an environment and observing the CI failure
- [ ] 7.4 Verify the `depends` closure is load-bearing: temporarily disable it, confirm `build-mobile` generates without `ruby`, and confirm a `fastlane` invocation then fails at run time rather than at config load — the failure mode the closure exists to prevent
- [ ] 7.5 Verify `locked = true` refuses an unlocked install: add a tool to a configuration without regenerating that environment's lockfile and confirm installation fails rather than resolving the version afresh
- [ ] 7.6 Re-verify the provenance negative case against the current mise — a deliberately wrong `slsa_signer_identity` on `github:getsops/sops` must fail with a signer-identity mismatch, not install
- [ ] 7.7 Walk each scenario in `specs/toolchain/tool-projection/spec.md` and `specs/toolchain/lockfile-integrity/spec.md` and confirm a task above exercises it; record any scenario that is asserted but never executed rather than leaving it assumed
