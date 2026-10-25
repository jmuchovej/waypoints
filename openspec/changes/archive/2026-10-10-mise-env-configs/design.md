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
the base. `null` keeps it out of mise altogether (D4).

**Why not the inverse** (list the environments a tool is excluded from): that is
the deny-list this change exists to remove, relocated. **Why not a single
`env` string**: several tools genuinely belong to more than one environment —
`just` is driven by `devtools`, both build lanes and `website`, and `sops` by
every lane that handles a secret — and forcing those into the base would put
them back in every CI job.

The existing `formatter` boolean is deleted rather than kept as sugar for
`envs = [ "devtools" ]`. It has no other meaning, and one way to express
membership is worth more than the shorthand.

### D2 — Environments name units of work, and compose

Because `MISE_ENV` takes a list (see Context), an environment does not have to
be a complete description of a machine's needs. It can name one job of work, and
a consumer names the jobs it does. That is a better decomposition than "local vs
automated", because the question a job can actually answer about itself is _what
it runs_, not _who is running it_.

- `xtask` — the release-automation lane: what the `cargo xtask` planner jobs and
  the draft-the-release job between them need.
- `test` — the test and audit runners.
- `devtools` — every formatter _and_ every check: treefmt and the formatters it
  drives, plus the rest of the `hk` suite (`hk`, `zizmor`, `lychee`,
  `cargo-deny`). A superset of what `formatter = true` meant. Selected by a job
  that runs a formatting or lint pass.
- `build-desktop` — the desktop bundling toolchain.
- `build-mobile` — the mobile toolchain, where `fastlane` and `xcodegen` live.
- `website` — the Cloudflare Worker lane: `wrangler`, `deno`, and the
  `sops`/`yq` pair `website/justfile` renders `.dev.vars` and pushes Worker
  secrets with.

There is no `development`; a contributor working without nix names the lanes
they work in (D3).

`website` was added after the job→tool audit (task 5.1), which found `wrangler`
with no consumer in any generated job — it is driven from `website/justfile` by
hand. The alternative was leaving it in the base, which would install a
Cloudflare client into every test and build job.

**`devtools` rather than `format`.** The formatters never travel alone: `hk`
runs treefmt, zizmor, lychee and `just deny` in one pass, so a `format`
environment and a `check` environment would always be selected together. One
name for both is the honest decomposition, and it is what the audit found —
every one of those tools is driven from `hk.pkl`, not from a job.

### D11 — The base is the common working set, not the intersection

An earlier draft read "base = what every environment needs" strictly, computed
the intersection, found it empty, and emitted a base with no `[tools]` at all.
That is defensible arithmetic and the wrong answer: it made a bare
`mise install` useless, so a consumer who did not know to set `MISE_ENV` got
nothing and no hint why — the same contributor D3 had just stopped serving with
a union environment.

The base instead carries what the repository's own scripts need, since every
way of launching, compiling or maintaining waypoints goes through a `just`
recipe and those recipes reach for the same handful of tools:

```
just  cargo-tauri  node  pnpm  nushell  sops  age  secretspec  yq
```

(plus `rust`, which arrives from `rust-toolchain.toml` through
`idiomatic_version_file_enable_tools` rather than `[tools]`.)

An environment then carries only what is genuinely specific to its lane, which
is a much smaller claim than the first partition made: `devtools` is the
formatters and checks, `build-mobile` is `fastlane`/`xcodegen`/`ruby`, `website`
is `wrangler`/`deno`, `test` is `cargo-nextest`, `xtask` is `git-cliff`, and
`build-desktop` is empty because the base already is the desktop build set.

**This is also why `sops` and `age` are base tools** rather than being listed
across the lanes that handle secrets: `secretspec run` wraps every build recipe
and `website/justfile` renders `.dev.vars` with `sops` and `yq` together, so
they are part of the working set, not a lane's specialism. `age` tracks `sops`
by referencing its `envs` directly, so the two cannot drift apart.

The cost is that `xtask` and `test` jobs install nine tools where five would
do. That is the price of a base worth having, and it is paid once per runner
rather than per lane.

**`git-cliff` goes in `xtask` rather than an environment of its own.** The
audit's awkward case: `create-release` invokes `git-cliff` and nothing else
through mise, and no environment named release work. A seventh `release`
environment for one tool buys a generated file and a lockfile; folding it into
`xtask` costs the name some precision, which the description above now carries.
The three jobs in question — `release-plan`, `prepare`, `create-release` — sit
in the same workflows and are one lane in practice.

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

**Every declared environment gets a file, member or not.** `build-desktop` is
the case that forced the decision: once the base carries the common working set
(D11), a desktop build needs nothing beyond it, so the environment's membership
is empty. An earlier draft emitted no file for that — mise treats a missing
`mise.<env>.toml` as nothing to add, so it would have worked — but it makes the
job wiring conditional on the current contents of a tool set. Emitting
`[tools]` with nothing in it means every build job carries the same `MISE_ENV`
shape whatever its lane currently holds, and the day a desktop-specific tool
appears it lands in a file the jobs already select. Verified: mise loads the
empty config without complaint, listing it in `mise config` as `(none)`.

### D3 — There is no union environment; a consumer selects the lanes it works in

Rejected after building it. An earlier draft gave each environment a `composes`
list and declared `development` as composing every other one, so its generated
file was their union — the argument being that the audience for `development` is
the contributor who is _not_ in the devenv shell, so nothing is there to export
`MISE_ENV` for them.

That argument does not survive contact with the fact that such a contributor has
to be told to install mise in the first place. Being told `MISE_ENV` alongside
it costs one sentence. Against that one sentence, `composes` bought: a submodule
type, a depth-bounded recursion to expand it, a 31-tool generated config, and a
lockfile for that config to be regenerated and reviewed with every toolchain
move. `composes` had exactly one user, and that user had exactly one member.

So `environments` is now a plain list of names, and composition happens where
mise already does it — at selection, from a comma-separated `MISE_ENV`. A
default is still expressible without a union file if one is ever wanted:
`.config/miserc.toml` is committed and mise honours an `env = [ ... ]` key
there, verified in task 1.1.

### D4 — `depends` is declared and _asserted_, never closed over

Tools gain a `depends` list projected into their mise entry: `gem:fastlane` and
`gem:rufo` depend on `ruby`, `npm:oxfmt`, `npm:wrangler` and `aqua:pnpm/pnpm` on
`node`. Each is written as the dependency's **spec** — read off the other tool's
own declaration, so a rename cannot leave a dangling string — and projected as
mise's displayed name (`ruby`, not `core:ruby`), which is what mise matches.

**The invariant mise will not enforce**: a config naming a `depends` target that
is not present resolves the dependent tool normally and exits zero — no error,
no warning (see Context). An environment split that let this through would
produce configs that look correct, pass every mise check, and fail at run time
with an interpreter not found — the same silent-no-op failure as the stale
`MISE_DISABLE_TOOLS` entry, in a new place.

**So every lane is asserted closed, rather than closed automatically.** An
earlier draft grew each environment's member set over `depends` until it stopped
changing, pulling missing runtimes in. That was replaced because:

- It made an environment's contents something other than what it declared.
  Reading `envs` no longer told you what the file held, and the extra members
  appeared only in generated output.
- It became redundant the moment the partition was real. `node` and `ruby` need
  explicit `envs` anyway — `envs = [ ]` means _base_, so without a list they land
  in the config every lane loads, and `xtask` and `test` install a node and a
  ruby neither runs. With those lists written, the closure computed nothing.

The assertion checks each lane — the base and every environment — for every
member's dependencies, resolving against that lane plus the base, since an
environment config is loaded on top of the base. A miss names the lane, the
dependent tool, and the edit:

```
waypoints.mise.devtools: `npm:oxfmt` depends on `core:node`, which is not a
member. Add `"devtools"` to that tool's `envs`.
```

A `depends` entry that is not some projected tool's spec is a separate
assertion.

#### `envs = null` replaces `include = false`

A tool is projected exactly when it says where it belongs, so the second flag
was never load-bearing — `include = true` is nowhere written in the tree, and
could not be, since it is what `envs` already implies. `null` means the tool
lives in the devenv shell and has no mise entry: the nix-only linters
(`deadnix`, `statix`, `nixf-diagnose`), plus `swift` and `kotlin` (D10).

### D10 — `kotlin` is nix-only; `swift` is removed

The audit (5.1) found both with no consumer anywhere — and the obvious guess for
each is wrong in the same way, so it is worth writing down:

- **Neither compiles its platform's target.** `iOS.justfile` prepends Xcode's
  toolchain to `PATH` precisely _so_ Swift resolves to Apple's and not to the
  Nix one. Android's Kotlin compiler arrives as a Maven artifact on Gradle's
  buildscript classpath —
  `classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:${libs.versions.kotlin.get()}")`
  in `gen/android/build.gradle.kts` — and runs embedded on the JDK. Gradle
  never looks for `kotlinc` on `PATH`, so `build-mobile` wants no Kotlin entry.
- **Neither formats anything either.** `treefmt.toml` does declare a
  `swift-format` formatter, but `swift-format` only joined the Swift toolchain
  in 6.0 and `core:swift` pins 5.10, so putting `swift` in `devtools` would
  install a toolchain that cannot supply the one thing it was wanted for.
  `ktfmt` is a standalone binary that needs no `kotlinc`.

`kotlin` gets `envs = null` — in the devenv shell, absent from mise. **`swift`
is deleted outright**, along with `languages.swift.enable`: its tool entry only
re-stated what that option already put in the shell, so with the entry gone the
option had no reader either. Measured before removing it: `SDKROOT` and
`DEVELOPER_DIR` come from `clangStdenv`'s apple-sdk rather than from
`languages.swift`, so the `env -i` the iOS build uses to clear them is
unaffected, and `cc` still links. `swift` and `sourcekit-lsp` now fall through
to whatever the host has. `treefmt`'s `swift-format` entry stays — it is
forward-looking for a generated iOS project, and `allow-missing-formatter`
already covers its absence, as it has all along.

**The `kotlin` entry is nonetheless load-bearing**, which `envs = null` hides:
`nix/devenv/android.nix` reads `tools.kotlin.package.version` into the
generated `libs.versions.toml`, and that is what pins the Gradle plugin above.
It is also what puts `kotlinc` in the shell, since `languages.kotlin` exposes no
`.package`. The entry carries a comment saying so, because "projected into
nothing" reads like "unused". `swift`'s entry has no such role — the toolchain
is already in the shell from `languages.swift.enable`, so that entry is
genuinely dead weight.

(`age` and `deno` were in this group on the first pass. They have homes now —
`age` tracks `sops` through the secret-handling lanes by referencing its `envs`
directly, so the two cannot drift; `deno` sits in `website`, where
`package.json` names it as node's alternative runtime.)

Leaving them at `envs = [ ]` is what the base is _for_ syntactically and wrong
semantically: the base is what every environment loads, so an unused tool there
is installed by every CI job. `envs = [ "development" ]` says the true thing —
a contributor without nix may want the runtime; no automated job does — and
`allow-missing-formatter = true` already covers the formatters mise cannot
provide.

### D5 — Generated files are derived, one `files` entry per environment

`files` becomes a computed attrset: the base entry plus, for each non-empty
environment, `.config/mise.<env>.toml` and a seeded `.config/mise.<env>.lock`.
An environment's file holds exactly the tools that named it — nothing is pulled
in (D4), so the file and the `envs` lists say the same thing.
Settings (`experimental`, `lockfile`, `idiomatic_version_file_enable_tools`) and
`min_version` stay in the base only — an environment file carries `[tools]` and
nothing else, so there is one place where toolchain policy is written.

`[env]` likewise stays in the base. It holds `PROJECT_ROOT` (from
`{{ config_root }}`) and an `_.file` pointer at `.env`, which carries the
`TAURI_*`/`TREEFMT_*` paths for mise and devenv alike. Those describe the
repository, not a mode of working in it, so no environment overrides them —
and keeping the pointer in one place means an environment file never has to
know the env file exists.

### D6 — mise config splits three ways: discovery, settings, derived

The dividing line is not "policy vs generated" but **what nix actually
computes**. Only derived values are projected; constants are hand-maintained
where they can be linted.

| file                            | holds                                       | why not elsewhere                                                     |
| ------------------------------- | ------------------------------------------- | --------------------------------------------------------------------- |
| `.config/miserc.toml`           | `auto_env`, `env_conf_d`, `env`             | read before config loading, so devenv's shell-entry write is too late |
| `.config/mise/conf.d/base.toml` | `[settings]`                                | constants, and lintable — see below                                   |
| `.config/mise.toml` (generated) | `min_version`, `[tools]`, `[env]`, `[vars]` | every value derives from the nix package set                          |

**Generated config is unlintable config.** `.config/treefmt.toml` excludes
`mise.toml` and `*/mise.toml`, and must: tombi would reformat what devenv
writes, and `check-generated` would then never converge. tombi binds a schema
through a `#:schema` comment, which devenv's TOML generator cannot emit — it
serialises an attrset. So anything projected into `mise.toml` gets no schema
validation at all. (`$schema` as a key is not an alternative: mise warns
`unknown field` on it, in `mise.toml` and `conf.d` alike.)

Since no setting derives from the package set — `min_version` does, but it is
top-level, not a setting — moving `[settings]` out costs nothing and buys
linting on the keys where a silent typo is most expensive.

#### `.config/miserc.toml` accepts discovery keys only

Measured: `locked`, `locked_verify_provenance`, `minimum_release_age` and
`experimental` placed there all read back as their defaults, with no warning of
any kind, while the same keys in a `[settings]` block take effect.

That is the opposite of this decision's first draft, which put the security
posture in `miserc.toml` on the assumption it was a general settings file. The
consequence of being wrong was silent: `locked` and `locked_verify_provenance`
read `false` while appearing to be set — exactly the failure shape this change
exists to remove, and unnoticed because nothing asserted the settings were in
force.

`mise settings get` cannot be used to test a discovery key. It reads the
settings subsystem, which `miserc.toml` does not feed, so it reports
`not set`/`Unknown setting` for `auto_env` and `env_conf_d` even while both are
working. It also misreports list-typed settings —
`idiomatic_version_file_enable_tools` reads `[]` while genuinely `["rust"]`.
Discovery keys are testable only behaviourally, against a control: set
`env = [ "x" ]`, confirm `mise.x.toml` loads, remove the file, confirm it stops.

`safe` belongs in none of the three. mise refuses it outside a global config and
says so — the one key in this area that fails loudly.

#### Consequence

Two hand-maintained files sit outside `check-generated`'s remit, and each is
small and slow-moving: `miserc.toml` is two lines deciding how config is
discovered, `conf.d/base.toml` is one `[settings]` block. Both are reviewed as
diffs, and `conf.d/base.toml` is schema-checked on top. `PROJECT_ROOT` stays in
nix rather than joining them — the mise side needs `{{ config_root }}` and the
devenv side `config.devenv.root`, neither derivable from the other, so the
invariant is that the two agree and both declarations belong where a reader
sees them together.

### D7 — Locking is hand-invoked; shell entry does nothing to lockfiles

`tooling/bin/mise-lock` refreshes every environment's lockfile and refuses to
run without a GitHub token. Shell entry neither writes a lockfile nor checks
one.

**Why this now.** The task it replaces ran `mise lock` on entry and listed the
lockfile it wrote in its own `execIfModified`, so entry rewrote the file it was
watching, over the network, unattended. A rate-limited run writes a short
lockfile and exits zero, so the damage was silent; with `locked = true` it
surfaces at a CI install, the furthest point from the machine that caused it.
It fired repeatedly while this change was being written, twice obstructing the
verification of unrelated tasks.

It lives in `tooling/bin` rather than `nix/devenv/bin` because the latter is
for scripts nix deploys as devenv tasks, and this is deliberately not one.

**What `mise lock` does not do**, which took a reading of its help to notice:
it refreshes checksums and URLs for the platforms a lockfile _already carries_
and will not add one. A lockfile first written narrow stays narrow. Widening it
takes `mise lock --platform <list>`.

### D8 — No local lockfile verification

Considered and rejected. A coverage check — every declared tool present, with
an entry per target platform — was built, worked, and then removed.

`locked = true` already refuses an install the lockfile does not cover, on
whichever platform is short. `check-generated` already fails on any lockfile
diff. The only thing a local check adds is noticing, before a push, that a
platform you are not on is short.

Against that it needed an expected-platform set and a per-tool exemption table,
both hand-maintained, and neither derivable: the set of platforms mise _can_
lock is not the set this repo targets, and `rust-toolchain.toml` enumerates
build targets rather than mise hosts — it lists Android and iOS triples, where
mise never runs, and a Windows ARM triple the lockfile has no slot for. Worse,
`mise lock` cannot act on the check's findings, so the remedy it recommended
could not fix what it reported.

If a view of lockfile state is ever wanted, `mise lock --dry-run --json`
reports mise's own rather than parsing the file.

### D9 — Jobs carry `MISE_ENV`; `WorkflowContext` carries nothing

`without_formatters(job, formatters)` becomes an explicit environment on each
job. `WorkflowContext.formatters` and `utils::mise::formatters()` are deleted;
`utils::mise::version()` stays, because CI must still install the mise that
wrote the lockfiles.

The job→environment mapping is decided once, per job, reading the existing job
definitions — not inferred. Desktop bundling names `build-desktop`; the mobile
lanes name `build-mobile`; test jobs name `test`; the planners that run
`cargo xtask` name `xtask`. No job names `development` — that environment exists
for a human without nix (D3), and a job naming it would be reinstating the
all-tools install.

**A job with no mise gets no `MISE_ENV`.** `check-generated` and
`devenv-update` install devenv through nix and reach their tooling from the
devenv shell; that is the appropriate shape for them, since what they check is
precisely devenv's own output. An earlier draft of this decision gave
`check-generated` `xtask,format`, which had nothing to act on — those jobs run
no mise-action, so there is no install for an environment to narrow.
`publish-release` likewise uses only the runner's `gh`.

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
