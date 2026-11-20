## Why

The tree's generated files are listed in three places that are maintained by hand and already disagree. The `check-generated` recipe in the `justfile` holds three binding paths, four icon pathspecs, and the notices path; `tooling/xtask/src/workflows/check_generated.rs` holds `OUTPUTS`, the same set as globs, for the CI trigger; and the two spell the Android icons differently — the recipe's `res/mipmap-*/ic_launcher*` and `values/ic_launcher_background.xml` against `OUTPUTS`'s `res/**/ic_launcher*`. Neither knows the group file's schema, which a unit test holds instead. Every generator already states what it owns — the workflow sweep, `typegen::Exports`, the icon manifest's `owned` patterns, `license::NOTICES_PATH`, `release_groups::SCHEMA_FILE`, `devenv::managed_files` — but two of those statements are unreadable to the check: the binding paths are joined inside the bin task, and the icons' are computed by Typst at render time. The same shape repeats for names the tooling shares: `wp-botswain[bot]` is a constant in `release_groups.rs` and again in `workflows/_helpers.rs`; `.github/workflows` is spelled in four modules; `main`, `Cargo.lock`, and the cargo-about configuration each appear in two or three.

The repository already has the rule and the precedent. `toolchain/tool-projection` requires that the lockstep check's file set be derived from the generator rather than enumerated by hand, and the constants each `license` submodule once defined were lifted into `xtask::license` so there is one `POLICY`, one `HOLDER`, one `NOTICES_PATH`. This change applies both to the rest of `xtask`. Why now: the CLI restructuring just regrouped the task files and added `ls managed`, which lists only devenv's files under a name that promises every generated file; and `tag-driven-releases` is drafted to add `.github/actions/**` to the generated outputs and to the devenv-update paths, a fourth copy unless there is one list to add it to.

## What Changes

- **A `constants` module.** `tooling/xtask/src/constants.rs` defines, once, every path and name that more than one module reads: the app's name, manifest, and group; the root manifest and the lockfile; the group file and its schema; the bot's login and git identity; the bot's branch prefixes and the `main` branch; the workflows directory and the OpenTofu variables file; the licensing inputs the checks read; the release index and stage directories. The modules that defined them import from there, and the second definitions go (design D1, D6).
- **A ledger of generated outputs.** `constants` carries one entry per generator — workflows, bindings, icons, notices, the group-file schema, devenv — naming the sources it reads and the outputs it owns as globs; devenv's outputs alone are read from `devenv info` at run time, as today. `check_generated.rs` derives its trigger paths and its diff set from the ledger and keeps no list; `ls managed` prints the ledger's outputs, devenv's included; the `check-generated` recipe diffs what `ls managed` prints and keeps no list (design D2, D5).
- **Generators hold to their entry.** The icon generator refuses to place or prune a path outside its ledger roots, so the coarse roots the ledger declares are checked on every run against the fine patterns Typst computes; the bindings task writes to the ledger's paths; the schema joins the lockstep check (design D3, D4).
- **No generator's output changes.** The bytes every generator produces are the same; the recipe's answer is the set it diffs today plus the schema; the regenerated `check-generated.yaml` differs only in its trigger's path list.

## Capabilities

### New Capabilities

- `toolchain/generated-outputs`: what the tree carries that is generated — one ledger naming each generator's sources and outputs, from which the lockstep check, its trigger, and the listing derive; generators hold to their declared roots; and the tooling's shared names and paths have one definition. The `toolchain` registry lists its areas by example, and none of the existing ones owns the lockstep check as a whole: `tool-projection`, `generated-bindings`, `app-icons`, and `third-party-code` each require their own output be covered, and that is where those requirements stay. So this change adds the area rather than stretching one of them.

### Modified Capabilities

None. `toolchain/tool-projection`'s _Generated configuration is committed and verified in lockstep_, `toolchain/generated-bindings`' _Committed bindings are covered by the lockstep check_, `distribution/app-icons`' _Generated icons are committed and held in lockstep with their sources_, and `licensing/third-party-code`'s _A third-party notices file is generated and committed_ stay as published: each still holds, and the new capability is how the check learns the set they name.

## Impact

- `tooling/xtask/src/constants.rs` (new): the shared constants and the ledger; `lib.rs` declares it.
- `tooling/xtask/src/{workspace,release_groups,semver_check}.rs`, `planners/release.rs`, `releases/{version,index,artifacts}.rs`, `workflows.rs`, `workflows/{_helpers,_steps,pull_request,release_index,check_generated,devenv_update}.rs`, `license/{headers,notices}.rs`, and the `license` block of `lib.rs`: definitions move out, imports move in.
- `tooling/xtask/src/workflows/check_generated.rs`: `SOURCES` and `OUTPUTS` become folds over the ledger.
- `tooling/xtask/src/icongen.rs`: the placed and pruned paths are held to the ledger's icon roots.
- `tooling/xtask/src/bin/xtask/tasks/{generate/bindings,ls/managed,release/{plan,index}}.rs`: write to, print from, and name through the constants.
- `justfile`: `check-generated` diffs `ls managed`'s output and regenerates the schema beside the bindings, icons, and notices.
- `.github/workflows/check-generated.yaml`: regenerated; the diff is the trigger's path list.
- `openspec/changes/tag-driven-releases/tasks.md`: its task adding `.github/actions/**` to the generated outputs reads the ledger rather than `OUTPUTS`, since it lands after this change.

## Non-goals

- No change to what any generator reads or writes, or to the bytes it produces.
- No folding of the per-tool exclusion lists — treefmt, typos, hk, tombi, Zed, the SPDX header check — which exclude generated files for reasons of their own and intentionally differ.
- No move for a constant one module alone reads: a cron expression, a bucket name, a header name stays beside its reader.
- No rewriting of test fixtures: a fixture that spells `src-tauri/Cargo.toml` is a foreign file's content, not a shared name.
- No `generate all`; the recipe keeps calling each generator by name.
- No change to the library's module names or to the command line.
