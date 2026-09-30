## Context

See `proposal.md` — Why. The relevant current-state facts, verified against the
pinned mise (`pkgs.mise`, projected as `min_version`):

- **Config environments are additive, never subtractive.** With `MISE_ENV=ci`,
  mise loads `.config/mise.toml` _and_ `.config/mise.ci.toml`. A tool declared in
  the base cannot be removed by an environment. Measured: a tool declared only in
  the base stayed active under `MISE_ENV=ci`, and the total tool count was
  identical with and without the variable.
- **Lockfiles follow the same additive rule.** `mise lock` with `MISE_ENV=ci`
  reports targeting both `.config/mise.ci.lock` and `.config/mise.lock`. Each
  environment adds a lockfile; it does not replace the base one.
- **`auto_env` is an early-init setting** — it must be read before any config
  file is loaded, so it cannot live in `.config/mise.toml`. It works from
  `.config/miserc.toml`, confirmed by `mise config` showing an environment file
  being loaded with only that setting present.
- **`auto_env` also activates platform environments implicitly** (`unix`,
  `<os>`, `<os>-<arch>`). Those names are therefore reserved: a generated
  environment must never collide with one.
- **`MISE_ENV` takes a comma-separated list and composes.** Measured with two
  scratch environments: `MISE_ENV=alpha,beta` loaded `mise.toml`,
  `mise.alpha.toml` _and_ `mise.beta.toml`, and all three tools resolved.
  Environments are therefore composable units, not mutually exclusive profiles.
- **`depends` matches a tool's displayed name, and `core:` is stripped.** With
  `"core:ruby"` as the config key, `depends = ["ruby"]` resolves — mise lists
  the tool as bare `ruby`.
- **An unsatisfiable `depends` is silently ignored.** A config declaring
  `"gem:rufo" = { depends = ["ruby"] }` with no `ruby` anywhere listed `rufo`
  normally and exited zero — no error, no warning. mise does not enforce the
  dependency closure.
- **The post-lock assertion in `sync-mise` checks resolution, not coverage.** It
  compares declared tool names against `mise ls --current`. A lockfile missing
  most of its platform entries still resolves every tool, so the assertion
  reports success. Observed: a rate-limited run wrote 144 of 224 platform
  entries and printed both `✓ Updated 144 platform entries (80 skipped)` and
  `all 31 declared tools resolved`.

## Goals / Non-Goals

Beyond the proposal's scope, at the design level:

**Goals**

- One generator. Base and per-environment files come from the same `tools`
  attrset and the same nix package versions — a version appears in exactly one
  place in the source, however many files it is written into.
- Environment membership is a property of the tool, not of the file. Nothing
  hand-lists file contents.
- A job's environment list is a claim about what the job does, readable in the
  generated YAML without cross-referencing anything.
- No environment can contain a tool without the runtime that tool needs. The
  generator guarantees this; mise does not.

**Non-Goals**

- Minimising duplication _across generated files_. A tool in two environments is
  written into two files. That is the format's shape, and generated duplication
  costs nothing.
- Deriving the partition automatically from job definitions. The mapping from
  job to environment is a human judgement recorded once, not inferred.

## Decisions

### D1 — Environment membership is an `envs` list on the tool; empty means base

`waypoints.mise.tools.<name>.envs` is a list of environment names. Empty (the
default) puts the tool in `.config/mise.toml`, which every environment loads. A
non-empty list writes the tool into each named environment's file and _not_ into
the base.

**Why not the inverse** (list the environments a tool is excluded from): that is
the deny-list this change exists to remove, relocated. **Why not a single
`env` string**: several tools genuinely belong to more than one environment —
`treefmt` is needed by local formatting and by `check-generated` — and forcing
those into the base would put the whole formatter set back in every CI job.

The existing `formatter` boolean is deleted rather than kept as sugar for
`envs = [ "format" ]`. It has no other meaning, and one way to express membership
is worth more than the shorthand.

### D2 — Environments name units of work, and compose

Because `MISE_ENV` takes a list (see Context), an environment does not have to
be a complete description of a machine's needs. It can name one job of work, and
a consumer names the jobs it does. That is a better decomposition than "local vs
automated", because the question a job can actually answer about itself is _what
it runs_, not _who is running it_.

- `xtask` — the lean set `cargo xtask` needs to generate and check.
- `test` — the test and audit runners.
- `format` — treefmt and everything it drives. What `formatter = true` meant.
- `build-desktop` — the desktop bundling toolchain.
- `build-mobile` — the mobile toolchain, where `fastlane` and `xcodegen` live.
- `development` — the union, for a contributor working without nix (D3).

**`ci` is deliberately dropped.** It named an actor, and once environments
compose, an actor is exactly the wrong unit: a CI job that builds the desktop
app and one that lints workflows share nothing, so a single `ci` set is the
union of everything automation ever does — the all-tools default this change
exists to escape. Jobs name `xtask`, or `build-desktop`, or `xtask,test`.

Declared as an enum in the module so a typo in `envs` is a nix evaluation error
rather than a file that silently never loads.

`macos` is deliberately _not_ an environment. OS restriction stays inline on the
tool as `os = [ ... ]`, which keeps a tool's platform gate next to its version
instead of splitting one tool across two files — and avoids colliding with the
platform environments `auto_env` activates on its own (see Context).

`development` rather than `dev`: the prototype's short name was expedience, and
these names are read far more often than typed.

An environment whose membership turns out empty emits no file. mise treats a
missing `mise.<env>.toml` as nothing to add, so `MISE_ENV` pointing at an
unmaterialised environment degrades to base-only rather than failing.

### D3 — `development` is generated as a union, not maintained as a superset

Environments compose at _selection_ time but not at _declaration_ time — there
is no include directive in a mise config. So an environment declaration gains a
`composes` list, and `development` is declared as composing every other
environment. Its generated file is the union of their members.

Without this, `development` is a hand-maintained superset that silently falls
behind every time a tool joins `build-mobile` — the same drift this change is
removing elsewhere. The alternative considered was emitting no `development`
file and having the shell export `MISE_ENV=xtask,test,format,...`; rejected
because the audience for `development` is precisely the contributor who is _not_
in the devenv shell, so there is nothing to do the exporting.

### D4 — `depends` is declared, and the generator closes over it

Tools gain a `depends` list projected into their mise entry: `gem:fastlane` and
`gem:rufo` depend on `ruby`, `npm:oxfmt` and `npm:wrangler` on `node`.

The generator computes the transitive closure of `depends` within each
environment and pulls any missing dependency into that environment's file. So
declaring `fastlane` in `build-mobile` brings `ruby` with it, and no environment
can contain a tool without the runtime it needs.

**This has to be the generator's job, because mise will not do it.** A config
naming a `depends` target that is not present resolves the dependent tool
normally and exits zero — no error, no warning (see Context). An environment
split without closure enforcement would therefore produce configs that look
correct, pass every mise check, and fail at run time with an interpreter not
found. That is the same silent-no-op failure as the stale `MISE_DISABLE_TOOLS`
entry, reintroduced in a new place.

A `depends` entry naming an undeclared tool is a nix assertion failure. mise
normalises `core:` away, so the projection emits the displayed name (`ruby`, not
`core:ruby`) while `envs`/`depends` are written against the module's tool names.

### D5 — Generated files are derived, one `files` entry per environment

`files` becomes a computed attrset: the base entry plus, for each non-empty
environment, `.config/mise.<env>.toml` and a seeded `.config/mise.<env>.lock`.
Settings (`experimental`, `lockfile`, `idiomatic_version_file_enable_tools`) and
`min_version` stay in the base only — an environment file carries `[tools]` and
nothing else, so there is one place where toolchain policy is written.

`[env]` likewise stays in the base. It holds `PROJECT_ROOT` (from
`{{ config_root }}`) and an `_.file` pointer at `.env`, which carries the
`TAURI_*`/`TREEFMT_*` paths for mise and devenv alike. Those describe the
repository, not a mode of working in it, so no environment overrides them —
and keeping the pointer in one place means an environment file never has to
know the env file exists.

### D6 — `.config/miserc.toml` is committed and hand-maintained

It cannot be generated: mise reads it before config loading, and devenv's
`files` writes on shell entry, which is already too late for the first read in a
fresh clone. It is also policy rather than derivation —
`locked_verify_provenance`, `minimum_release_age`, `safe` are security posture
decisions that should be reviewed as a diff, not recomputed from the package
set.

Consequence: `miserc.toml` is outside `check-generated`'s remit, and a
contributor editing it is editing policy deliberately. That is the intent.

### D7 — Locking is an explicit recipe; shell entry verifies

The `mise:lock` devenv task is split:

- **On shell entry**: a check that each committed lockfile is present and covers
  every tool its config declares, on every platform in the target set. No
  network, no writes. A short or stale lockfile fails here, loudly, in the shell
  that produced it.
- **`just mise-lock`**: regenerates every environment's lockfile, in one run,
  deliberately, with a token.

**Why this now.** The present task lists `.config/mise.lock` in its own
`execIfModified`, so entering the shell regenerates the file it is watching.
When GitHub rate-limits the unauthenticated path, mise writes a truncated
lockfile and exits zero. Splitting one lockfile into four multiplies the blast
radius, and `locked = true` with `locked_verify_provenance = true` moves the
failure from the shell to a CI install — the furthest possible point from the
machine that caused it.

**Alternative considered**: keep automatic regeneration but fail on a short
lockfile. That still requires the coverage check, and it still performs
unattended network writes on every shell entry; the check alone is the useful
half.

### D8 — The coverage assertion counts platform entries, not just resolution

Today's assertion passes on a lockfile missing a third of its entries. The
replacement compares, per environment, the declared tool × target-platform
product against what the lockfile actually holds, with a declared exemption list
for backends that legitimately produce no per-platform entries (`gem:`, `npm:`,
and `rust`). An exemption is a named entry, not a silent skip — an unexplained
gap is a failure.

### D9 — Jobs carry `MISE_ENV`; `WorkflowContext` carries nothing

`without_formatters(job, formatters)` becomes an explicit environment on each
job. `WorkflowContext.formatters` and `utils::mise::formatters()` are deleted;
`utils::mise::version()` stays, because CI must still install the mise that
wrote the lockfiles.

The job→environment mapping is decided once, per job, reading the existing job
definitions — not inferred. Desktop bundling names `build-desktop`; the mobile
lanes name `build-mobile`; test jobs name `test`; workflow generation and the
generated-file check name `xtask` and `format`, composed as `xtask,format`. No
job names `development` — that environment exists for a human without nix (D3),
and a job naming it would be reinstating the all-tools install.

## Risks / Trade-offs

- **A tool moved out of the base breaks a job that quietly relied on it** →
  Every job's environment is chosen by reading what that job runs, and CI fails
  closed: with `locked = true`, a tool absent from the active config is not
  installed and the command is not found. The failure is immediate and names the
  binary.
- **Four lockfiles regenerate at once, quadrupling GitHub API traffic in a
  single run** → Locking becomes an explicit, authenticated recipe (D7) rather
  than something shell entry triggers, so the traffic happens once per
  deliberate act instead of once per shell.
- **`check-generated` must cover a set that can grow** → It already does. Both
  the `just check-generated` recipe and the generated workflow's trigger paths
  derive their file list from `devenv info`'s `# files` section
  (`utils::devenv::managed_files`, `cargo xtask devenv-files --copied`), which
  today reports `.config/mise.toml` and `.config/mise.lock` and will report each
  new environment file without any edit. `.config/miserc.toml` is not a devenv
  output and so stays outside the gate, which is correct (D6). Only
  `devenv_update.rs`, which names the two paths literally in the PR body and
  commit list, needs updating.
- **An `MISE_ENV` typo in a job silently gets base-only tooling** → mise does not
  validate environment names, so this cannot be caught by mise. The generator
  emits the value from a Rust enum matching the nix enum, so the typo cannot be
  written in the first place.
- **The `depends` closure quietly inflates an environment** → Pulling `ruby` into
  `build-mobile` for `fastlane` is correct but invisible in `devenv.nix`. The
  generated file is the record: it is committed, diffable, and checked by
  `check-generated`, so an unexpected inflation shows up in review rather than in
  a runner log.
- **`development` as a generated union re-creates the all-tools install it
  replaces** → For the nix-less contributor that is the point; they asked for
  everything. No CI job names `development`, and the enum makes that auditable
  with one grep over the generated workflows.
- **Environment membership is a second place to be wrong about a tool** →
  True, and unavoidable: the information did not previously exist anywhere, it
  was approximated by a deny-list. Being explicitly wrong in one place is an
  improvement on being implicitly wrong in twenty.

## Migration Plan

Sequenced so each step is independently verifiable:

1. Land `.config/miserc.toml` on its own. It changes mise's posture
   (`locked`, `locked_verify_provenance`, `minimum_release_age`) without moving
   any tool, so any fallout is attributable.
2. Add the `environments` declaration, the `envs` and `depends` options, and the
   generator — with every tool still defaulting to base, so every environment is
   empty and no environment file is emitted. Generated output is byte-identical
   apart from `[vars].WAYPOINTS_FORMATTERS` going away and `depends` appearing
   on four tools; `check-generated` proves it.
3. Split lockfile regeneration from verification (D7, D8) — still one lockfile.
   Verify the coverage assertion fails on a deliberately truncated lockfile
   before trusting it.
4. Move tools into environments, regenerate, and lock. Now there are several
   lockfiles and the assertion from step 3 is doing real work.
5. Switch jobs from `MISE_DISABLE_TOOLS` to `MISE_ENV`, delete the formatters
   plumbing, regenerate the workflows.

**Rollback**: steps 4 and 5 are the only ones with CI-visible behaviour, and
both are reverts of generated output plus the nix source that produced it. Step
1 is independently revertible. Nothing here is stateful — no published artifact
depends on the config partition.

**Ordering constraint**: the `MISE_ENV` switch must follow the partition. A job
naming `build-desktop` before `.config/mise.build-desktop.toml` exists gets
base-only tooling and fails on a missing binary.

## Open Questions

- Whether `xtask` and `test` stay separate or collapse into one. It depends on
  how much the test jobs need beyond what workflow generation needs, which the
  job→tool audit makes visible. Collapsing later is a file deletion and an
  `envs` edit — it changes no interface.
- Whether `build-desktop` and `build-mobile` want per-platform siblings, or
  whether inline `os = [ ... ]` covers it. Inline gating is the starting
  position (D2); only a tool needed on one platform _and_ absent on another
  within the same environment would argue otherwise.
- Whether `hk` belongs in `format` or stands alone, pending the separate
  question of its `packslip.sigstore.json` provenance (out of scope here).
