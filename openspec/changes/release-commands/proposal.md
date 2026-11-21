## Why

The `release` command line says how each task is wired rather than what it does. Nine subcommands sit under it — `prepare`, `plan`, `stage`, `upload`, `manifest`, `withdraw`, `index`, `land`, `groups` — and two of them are planners: `release prepare` decides one channel run and, on nightly, mints the day's tag, while `release plan` decides a release-cycle transition a person names by hand, so a dispatcher choosing `advance` on a series that has already gone stable is refused only after the fact. `land` hides four landing actions behind a word John found opaque; `stage`, `upload`, `manifest`, and `withdraw` are four acts on one set of files under four unrelated names; and the group file's generated schema lives under `release groups` because that is where its type is, though it is a generated output `version-integrity` holds to its source like every other.

The tag-driven pipeline John settled on 2026-11-20 and 2026-11-21 — recorded in `tag-driven-releases` — needs commands this line does not have: a `bump` that infers the transition from the state of the open series and refuses to move a tag over bookkeeping alone, a `describe` that reads a tag into a build's identity, a `record` that writes the index entry from inside the publishing run, and an `undo` that reverts a bump's write when its release is withdrawn. This change adds those and renames the rest, and it does so without touching the shape of any workflow: `release plan` and `release prepare` stay, unchanged in behavior, so the committed workflows keep running under the new spellings, and `tag-driven-releases` turns the workflows over on top of a command line that already exists and is tested. The sequence is `single-source-constants`, then this change, then `tag-driven-releases`; each lands alone so that one review reads one kind of change.

## What Changes

- **`release bump` plans every ref.** `release bump (--channel <nightly|preview|stable> [--major] | --group <name> [--floor <bump>]) [--dry-run [--into <dir>]]` is the planner the workflows will dispatch. For a channel it reads the open series from the remote and infers the transition: a preview advances a series in preview and cuts one from `main` when the last is stable; a stable promotes a series in preview, patches one that is stable and carries new cherry-picks, and refuses otherwise; nightly is today's nightly planner, sharing `plan_nightly` with `prepare`. On every channel a bump that finds only bookkeeping since the last tag moves no tag, as nightly already skips a quiet day; a promotion is exempt, since it moves no work. A version a branch declares that no tag names is a withdrawal in flight, and every bump refuses until its revert lands. Sizing is unchanged: only a cut reads `main`'s history for severity, `--major` is still the only way to a major, and advance, promote, and patch stay mechanical (design D1).
- **Landing is three verbs.** `release merge --pull-request <n> [--no-wait]` replaces `land auto-merge` and `land wait`, which every workflow ran back to back; `release tag --tags <list> --commit <sha>` and `release open-series --version <v> --commit <sha>` are today's `land tag` and `land open-series`, flattened. The in-run wait stays (design D2).
- **The store steps are `distributables`.** `release distributables {rename, upload, commit, withdraw}` replaces `release stage`, `upload`, `manifest`, and `withdraw`; `commit` is the manifest write, which is the publication's commit point. The library module follows the name (design D3).
- **Three commands arrive before their callers.** `release describe --ref <ref>` reads a tag, or a tree, into the version, channel, flavor, and store build codes a build stamps; `release record --tag <v>` is today's `release index`, renamed for the job that will run it after the manifest; `release undo --tag <v> --commit <sha> [--main]` reverts a bump's write into the working tree for the bot's pull-request action, and for a cut names the series branch and library tags to delete and reverts `main`'s write too. `version-integrity` admits a bot revert. Nothing calls the three yet (design D4).
- **`generate` owns the group file's generated outputs.** `generate schemas` replaces `release groups schema` and `generate versions` replaces `release groups project`; the `groups` group retires (design D5).
- **The old planners stay until the workflows turn over.** `release plan` and `release prepare` keep their behavior; `_jobs.rs` and `release_plan.rs` keep spelling them; every other generator spelling follows its rename and the workflows are regenerated with today's shapes, so `just check-generated` is clean after every task and no committed workflow calls a command that no longer exists (design D6).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `distribution/release-planning`: two requirements are **ADDED** — _A bump infers the transition from the series_ and _Bookkeeping alone moves no tag_. Their scenarios about a withdrawal in flight refer to the revert `tag-driven-releases` specifies; here the state is only refused. The open deltas of `library-minting`, `group-file-versions`, and `explicit-group-ownership` modify other requirements of this spec and are not touched, and `tag-driven-releases` adds and modifies others still.
- No other spec moves. `describe`, `record`, and `undo` have no caller in this change, so the pipeline's behavior is unchanged; the requirements they serve — a publishing run that records after its manifest, a withdrawal that undoes the bump — are `tag-driven-releases`' deltas to `distribution/release-publishing` and `distribution/release-index`.

## Impact

- `tooling/xtask/src/bin/xtask/main.rs` and `bin/xtask/tasks/release/`: `bump.rs`, `describe.rs`, `merge.rs`, `tag.rs`, `open_series.rs`, `record.rs`, `undo.rs`, and `distributables/{rename,upload,commit,withdraw}.rs` arrive; `land.rs`, `index.rs`, `groups.rs`, `stage.rs`, `upload.rs`, `manifest.rs`, and `withdraw.rs` go; `prepare.rs` and `plan.rs` stay; `generate/{schemas,versions}.rs` are new.
- `tooling/xtask/src/planners/release.rs`: the inference over the series state, the series reader over the remote, the withdrawal-in-flight reader; `planners/nightly.rs`: unchanged, `plan_nightly` shared by `prepare` and `bump`.
- `tooling/xtask/src/releases/artifacts.rs` becomes `releases/distributables.rs`.
- `tooling/xtask/src/version_integrity.rs`: admits a bot revert; names `generate schemas` for a stale schema.
- `tooling/xtask/src/workflows/{_steps,_jobs,release_plan,release_index}.rs` and the tests that pin command spellings: `release distributables …`, `release merge`, `release tag`, `release open-series`, `release record`; `release prepare` and `release plan` as today.
- `.github/workflows/*.yaml`: regenerated, today's shapes under the new spellings.
- `tooling/bin/xfastlane`, `src-tauri/fastlane/{AGENTS.md,Fastfile}`, `.config/treefmt.toml`, `docs/CONTRIBUTING.md`, the `check-generated` recipe where it names the schema command, and the active changes' documents spell the new commands.

## Non-goals

- No change to any workflow's shape, triggers, jobs, or steps beyond the spelling of the commands they run; that is `tag-driven-releases`.
- No deletion of `release plan`, `release prepare`, `Plan::tag_minted`, or the nightly planner's `should_build` and `dry_run` outputs; the committed workflows read them until `tag-driven-releases` replaces the workflows.
- No caller for `describe`, `record`, or `undo`; they are added, tested, and left for the workflows to dispatch.
- No change to the train, to how a bump sizes a version, or to the release test filterset.
- No change to the rulesets or to `tooling/infra/`; the deletion rule a cut's undo needs is `tag-driven-releases`' change, since nothing here deletes a branch.
- No `xtask`-minted App tokens and no `xtask`-authored commits: `undo` leaves the reverted write in the working tree for the pull-request action, as the planners leave theirs.
