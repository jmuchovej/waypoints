## Why

`nix/devenv/mise.nix` projects every tool the devenv shell provides into a single
`.config/mise.toml`. That file is the whole toolchain, so every consumer outside
the shell — every CI job — starts from "all of it" and subtracts. The subtraction
is spelled `MISE_DISABLE_TOOLS`, and devenv feeds it by projecting a
comma-joined `[vars].WAYPOINTS_FORMATTERS` string, which `xtask` reads back out
of the TOML (`utils::mise::formatters`) and stamps into each generated job
(`without_formatters`).

That round trip — Nix attrset → TOML var → Rust string → YAML `env:` on roughly
twenty jobs — is both fragile and already broken. The committed release workflows
disable `aqua:mic92/nixfmt-rs`, a spec that never resolved: aqua matching is
case-sensitive, and nixfmt is now sourced from `github:Mic92/nixfmt-rs` with the
owner cased as upstream spells it. The jobs are suppressing a tool that is not
declared and failing to suppress the one that is. Nothing caught it, because a
deny-list naming a nonexistent tool is silently a no-op.

Subtraction is also the wrong shape for the question. A release job does not want
"everything except formatters"; it wants `cargo-tauri`, `pnpm`, `node`, and
little else. There is currently no way to say that. Every job installs, verifies
provenance for, and pays cache weight on the full set minus a hand-maintained
exclusion list — `lychee`, `hk`, `zizmor`, `xcodegen`, `fastlane`, `git-cliff`
included, whether the job touches them or not.

mise's own answer is config environments: `MISE_ENV=<name>` loads
`.config/mise.<name>.toml` on top of the always-loaded base, each with its own
lockfile `.config/mise.<name>.lock`, and the variable takes a comma-separated
list that composes. Selecting environments is one variable, declared once per
job, that names what the job _does_ rather than enumerating what it is not.

## What Changes

- **Add an `envs` axis to `waypoints.mise.tools.<name>`.** A tool declares which
  environments it belongs to. The base `.config/mise.toml` keeps only the tools
  shared by every environment; everything else moves into the environment that
  needs it.
- **Shape environments by task, not by actor.** `xtask`, `test`, `format`,
  `build-desktop`, `build-mobile` — each naming a set of work, with
  `development` as the union for a contributor working without nix.
  `MISE_ENV` accepts a comma-separated list and loads every named file, so a job
  composes the sets it needs rather than picking one profile.
- **Record tool dependencies with mise's `depends`.** `gem:fastlane` and
  `gem:rufo` need `ruby`; `npm:oxfmt` and `npm:wrangler` need `node`. Declaring
  it lets the generator guarantee that an environment containing a tool also
  contains what that tool runs on — mise itself does not check this.
- **Generate one `.config/mise.<env>.toml` per declared environment** alongside
  the base, all from the same nix packages, so versions stay single-sourced.
  Hand-writing an env file would reintroduce the exact version-drift trap the
  projection exists to close.
- **Fold `formatter = true` into `envs`.** It exists today only to populate the
  deny-list; as membership of a `format` environment it needs no separate
  concept.
- **REMOVED:** `[vars].WAYPOINTS_FORMATTERS`, `utils::mise::formatters()`,
  `without_formatters()`, `WorkflowContext.formatters`, and the
  `MISE_DISABLE_TOOLS` line on every generated job. Jobs set `MISE_ENV` instead.
- **Commit `.config/miserc.toml`** as a hand-maintained file for mise's
  early-init settings — `auto_env`, `locked`, `locked_verify_provenance`,
  `minimum_release_age`, `safe`, `experimental`. These are read before any config
  file loads, so they cannot be projected into `.config/mise.toml`; `auto_env` in
  particular is what makes environment selection work at all.
- **Lock every environment.** `nix/devenv/bin/sync-mise` currently runs a bare
  `mise lock`, which writes only `.config/mise.lock`. It must lock each
  environment, and `check-generated` must cover every resulting lockfile —
  otherwise an unchecked lockfile becomes where drift hides.
- **Make lockfile regeneration explicit rather than automatic.** The `mise:lock`
  devenv task lists `.config/mise.lock` in its own `execIfModified`, so shell
  entry regenerates the file it watches, over the network, unattended. A
  rate-limited run writes a short lockfile and reports success — observed
  repeatedly. This is in scope because the split multiplies one such file into
  several and because `locked_verify_provenance` makes a short lockfile fail at
  install time in CI rather than in the shell where it was produced.
- **A partition need not be exhaustive.** If an environment ends up with no
  tools of its own, no file is emitted for it. What matters is that the
  mechanism exists and that any job can name the set it needs.

## Capabilities

This change alters no _product_ behavior, but it is not behaviour-free. The
invariants it establishes — that a version has one source, that an environment
is closed over its tools' dependencies, that a lockfile is complete only if it
covers every platform — did not previously exist anywhere. They were
approximated by a deny-list, and every failure this change fixes is one of them
being violated silently. Those are requirements, and they belong in a spec.

No registered domain covers them. `distribution` is packaging, installation, and
update delivery _of the product_; this is the repository's own reproducibility.
So this change mints **`toolchain`** and registers it in `openspec/config.yaml`,
alongside `distribution`, which `nix-flake-overlay` created on disk without ever
adding to that registry.

`toolchain` establishes the repository's own reproducibility: deltas in it never
constrain app behavior nor cite product domains, and no product domain cites it.

### New Capabilities

- `toolchain/tool-projection`: how tools are declared once and projected into
  consumable configuration — single-source versioning, environments as
  composable units of work, the dependency closure that keeps an environment
  internally complete, selection-not-exclusion as the consumer contract, and the
  boundary between generated projection and hand-maintained early-init policy.
- `toolchain/lockfile-integrity`: what makes a lockfile trustworthy — per-platform
  coverage rather than mere resolution, verification on shell entry versus
  explicit authenticated regeneration, and enforcement of the lock and its
  provenance at install time.

### Modified Capabilities

_None._ `distribution/nix-overlay` is untouched: it describes what the packages
are, not what tooling built them.

## Impact

- **`nix/devenv/mise.nix`**: the `tool-type` submodule gains `envs`; `formatter`
  goes away; the single `files.".config/mise.toml"` entry becomes a base plus a
  generated set of per-environment files and their seeded lockfiles. The
  existing assertion (a projected tool needs both `package` and `spec`) stays,
  and gains a companion: an `envs` entry must name a declared environment.
- **`devenv.nix`**: every `formatter = true` becomes an `envs` list; tools with
  no `envs` land in the base. This is where the actual partitioning decision
  gets made, tool by tool, and it is the part that needs review rather than
  mechanism.
- **`nix/devenv/bin/sync-mise`**: locks each environment rather than the current
  one, and its post-lock assertion — which today compares declared tools against
  `mise ls --current` — must be evaluated per environment. That assertion
  currently checks _resolution_, not platform coverage, which is why it passes
  on a truncated lockfile; that gap is what makes the verify-don't-regenerate
  item above load-bearing.
- **`tooling/xtask/src/utils/mise.rs`**: `formatters()` and `FORMATTERS_VAR` are
  deleted. `version()` (reading top-level `min_version`) stays — CI must still
  install the mise that wrote the lockfiles.
- **`tooling/xtask/src/tasks/workflows/_jobs.rs`** and **`workflows.rs`**:
  `without_formatters` is deleted and `WorkflowContext.formatters` with it; jobs
  gain an environment selection instead. Deciding which environment each job
  names is the substantive half of this work.
- **`.github/workflows/*.yaml`** (generated): roughly twenty `MISE_DISABLE_TOOLS`
  lines disappear and `MISE_ENV` lines replace them. The regeneration also
  corrects the stale `aqua:mic92/nixfmt-rs` entry as a side effect.
- **`.github/workflows/devenv-update.yaml`**: names `.config/mise.toml` and
  `.config/mise.lock` literally in its commit list, so `devenv_update.rs` needs
  the full set. `check-generated` needs no such edit — both its recipe and its
  trigger paths derive the list from `devenv info`, which reports every new
  environment file on its own.
- **Lockfile count and CI cost**: each environment's lockfile is an independent
  network-resolved artifact. Regenerating them all is more GitHub API traffic in
  one go than today's single lock — which is an argument for doing it
  deliberately from a `just` recipe, not on every shell entry.

## Non-goals

- Making mise provide dependencies _inside_ the devenv shell. Nix does that; the
  projection exists for consumers outside it, and that does not change.
- Making config environments subtractive. The base `.config/mise.toml` is always
  loaded, by design — it holds the foundational tools without which nothing in
  this repo can run. Environments add to it; they never mask it.
- Per-platform config files. OS gating stays inline on the tool as
  `os = [ ... ]`, which keeps a tool's platform restriction next to its version
  rather than splitting one tool across two files.
- Generating `.config/miserc.toml`. Early-init settings are hand-maintained and
  reviewed as policy, not derived from the package set.
- Adding, removing, or re-versioning any tool. The set and its versions are
  exactly what they are today; only their grouping changes.
- Changing what `check-generated` _means_ — it remains the gate that the
  committed generated files match what devenv and xtask would produce.
- Resolving `jdx/hk`'s `packslip.sigstore.json` provenance question, which is
  unrelated to config partitioning.
