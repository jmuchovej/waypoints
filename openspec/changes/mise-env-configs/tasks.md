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
- [x] 3.7 Prune environment configs whose environment no longer has members. devenv's `files:cleanup` cannot do this: it removes a file it has stopped generating only when that file is a symlink into the nix store, and the environment configs are `copyMode = "copy"` because CI reads their contents. A `mise:prune-environments` task removes any `.config/mise.<name>.{toml,lock}` outside the set devenv expects; verified that planted strays are removed, that a populated environment's files survive alongside them, and that emptying an environment cleans up its files
- [x] 3.6 Add the generated `.config/mise.*.toml` files to `.config/treefmt.toml`'s excludes and narrow the `[[schemas]]` glob in `.config/tombi.toml` to the hand-edited configs only; verify `just check-generated` converges across two consecutive shell entries. Schemas serve LSP-assisted edits — hand or agent — so generated files do not want one; a malformed tool table is caught by mise parsing it or failing loudly, which is the touchpoint that motivates generating it at all. Without the exclusion tombi indents `[tools]` entries while devenv writes them flush-left, and the check never converges

## 4. Lock verification vs regeneration

- [x] 4.1 Replace `sync-mise` with a hand-invoked `tooling/bin/mise-lock`. It lives there rather than `nix/devenv/bin` because that directory is for scripts nix deploys as devenv tasks, and this is no longer one; `tooling/bin` is on PATH, so `mise-lock` runs directly as well as through `just mise-lock`
- [x] 4.2 Remove the lock task from devenv entirely rather than converting it to a verifier. Verified no remaining task writes a lockfile — `mise:prune-environments` is the only mise task left — and that two consecutive shell entries leave every lockfile byte-identical
- [x] 4.3 Add a `just mise-lock` recipe that refreshes every environment's lockfile, mapping `GH_TOKEN` to `GITHUB_TOKEN`; verified it refuses with rc=1 and a stated reason when no token is present
- [x] 4.4 **Dropped — the check was not worth its weight.** `locked = true` already refuses an install the lockfile does not cover, on whichever platform is short, and `check-generated` already fails on any lockfile diff; a local check could only add noticing before a push that some other platform is short. Against that, it needed a hand-maintained expected-platform set and an exemption table, and `mise lock` cannot even act on its findings — it refreshes the platforms a lockfile already carries and will not add one, so the remedy the check recommended could not fix what it reported. `mise lock --platform` is how a platform gets added, and `--dry-run --json` is how mise reports its own view without parsing anything
- [x] 4.5 **Dropped with 4.4.** Recorded for the record: the check did work — truncating a lockfile produced `cargo-deny is not locked at all` and `just missing macos-arm64` at rc=1 — and the test was only possible after 4.2, because while the task still regenerated on entry it overwrote the truncation before the check could read it
- [x] 4.6 Only the PR body needed changing. `paths()` already chains `context.devenv_files` from `devenv info`, so the commit file list was derived all along and picks up each new environment config and lockfile without an edit

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
- [x] 7.8 **Moot.** The `core:swift` / `macos-arm64` exemption is gone with the coverage check. The underlying question stands and is worth knowing: swift locks only Linux platforms while node, deno and ruby all lock `macos-arm64`, yet `mise install --dry-run` on macos-arm64 reports `swift@5.10.1 would install`. If a job installs swift through mise on macOS under `locked = true`, that is where it surfaces
- [ ] 7.6 Re-verify the provenance negative case against the current mise — a deliberately wrong `slsa_signer_identity` on `github:getsops/sops` must fail with a signer-identity mismatch, not install
- [ ] 7.7 Walk each scenario in `specs/toolchain/tool-projection/spec.md` and `specs/toolchain/lockfile-integrity/spec.md` and confirm a task above exercises it; record any scenario that is asserted but never executed rather than leaving it assumed
