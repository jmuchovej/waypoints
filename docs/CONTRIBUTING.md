# Contributing to Waypoints

Both humans and agents contribute here. This document is the single source of
truth for how we write commits; `AGENTS.md` points agents at it.

## Why commits are strict

Commit messages are not just history. The `type` feeds changelog and
release-note generation; the `scope`'s **region** tells you where in the codebase
the change lives; and an optional **subscope** associates commits that belong to
one capability, so scattered work reads as a deliberate initiative rather than a
pile of disjoint changes. A sloppy or ad-hoc message drops a change out of the
notes or hides that association — so the format below is a contract, not a
suggestion.

## Format

```
type(scope): summary — optional elaboration   (#123)

Optional body. Prose, wrapped at ~72 columns. Explain *why* the change
exists and the trade-offs it makes — not a file-by-file list of what
changed (the diff already says that). State how you verified it.

BREAKING CHANGE: what broke and how to migrate.   ← footer, only if needed
```

## Types

Use exactly one of these. The strays that have crept into history
(`dev`, `deps`, `init`, `parked`, `fea`, `feeat`, `eat`, …) are retired as
_types_ — map them below. (`dev` survives as a _region_ — see Scope.)

| Type       | Use for                                                                    | Release-note section |
| ---------- | -------------------------------------------------------------------------- | -------------------- |
| `feat`     | a user-visible capability                                                  | Features             |
| `fix`      | a bug fix                                                                  | Fixes                |
| `perf`     | a performance improvement                                                  | Performance          |
| `refactor` | behavior-preserving restructuring                                          | (internal)           |
| `docs`     | documentation, comments, or specs only                                     | (internal)           |
| `test`     | tests only                                                                 | (internal)           |
| `style`    | formatting / whitespace only (was `fmt`, `styling`)                        | (internal)           |
| `build`    | build system, dependencies, or codegen tooling (was `dev` typegen, `deps`) | (internal)           |
| `ci`       | CI / workflow configuration                                                | (internal)           |
| `chore`    | maintenance not covered above (was `init`, `parked`)                       | (internal)           |
| `summary`  | a pull request's merge message when it spans release groups                | (skipped)            |

Add `!` after the scope for a breaking change: `feat(bedrock)!: …`, and include
a `BREAKING CHANGE:` footer.

`summary` is for merge messages only; see [Landing a pull
request](#landing-a-pull-request).

## Scope — `region[/subscope]`

The scope has two parts. The **region** (required, first segment) is drawn from a
fixed set and names where the change lives — so the scope alone always tells you
the area of the codebase. The **subscope** (optional, second segment) is
open-ended and names the capability the commit is part of.

### Region — the fixed set

A region maps to actual crates/areas; it is **not** a rollup of unrelated crates.
Do not invent regions (the one-offs `flux`, `vox`, `obx`, `things` are retired).

| Region       | Covers                                                                                                       |
| ------------ | ------------------------------------------------------------------------------------------------------------ |
| `domain`     | the `domain` crate — the core definitions                                                                    |
| `cartograph` | the `cartograph` crate — the core app engine                                                                 |
| `bedrock`    | the `bedrock*` crates **only** (storage engine)                                                              |
| `outpost`    | the `outpost*` crates **only** (sync/networking)                                                             |
| `store`      | the `store` data layer _(transitional — expected to fold into `cartograph`)_                                 |
| `transload`  | the `transload` crate — the Bundle format's export and ingest                                                |
| `app`        | the Tauri app + frontend                                                                                     |
| `website`    | the marketing/docs site                                                                                      |
| `dev`        | developer tooling — `tooling/xtask` (the Rust dev-toolchain: codegen, CI planning, the dev server), `devenv` |

For a repo-wide change, omit the scope. Special scopes `deps` (dependency bumps)
and `release` (release plumbing) may be used in place of a region.

### Subscope — associating a capability

A subscope names the capability a commit belongs to. Its job is **association**:
when one capability is built across several commits — often across several
regions — the shared subscope makes that explicit, so the commits read as "all
part of sync" instead of unrelated changes. Split the work one commit per region,
each carrying the subscope:

```
feat(outpost/sync):    admission policy — Owner-gated registration
feat(cartograph/sync): setup-code engine + QR generation in Rust
feat(app/sync):        onboarding, settings, and device-pairing UI

feat(cartograph/editor): reparent_mark with correct parent/order computation
feat(app/editor):        block context menu — delete, duplicate, convert-to
```

Anyone (or any tool) scanning the log can now see the `sync` and `editor`
initiatives as coherent bodies of work rather than disjoint commits. Coin a
subscope only when it associates work across two or more commits, and reuse it
consistently rather than minting one-offs. Subscopes are _capabilities_, not
directory paths — don't mirror the file tree.

### Combining and omitting

- **Cross-cutting work:** prefer one commit per region sharing a subscope (above)
  over a single sprawling commit.
- **Genuinely atomic multi-region change:** comma-join regions, only while every
  region falls in one release group — `refactor(app,store): …` is one `waypoints`
  commit. Rare. Across groups, the pull request lands as a `summary`.
- **Repo-wide change:** omit the scope — `chore: bump toolchain to rust 1.94`.

## Subject line

- Lowercase after the colon; **no trailing period**.
- Aim for ≤ 72 characters.
- An em-dash `—` introduces a short elaboration:
  `feat(outpost): device registry — visibility, attribution, connection status`.
- Issue and spec references go at the end: `(#132)`, `§10.2`.

## Body

Required for anything non-trivial. Write prose, not a changelog of the diff.
The bar the best existing commits already set:

> The classifier, the pairing URL envelope, and QR rendering move out of
> TypeScript into the one engine every platform shares … so a TS/Rust
> split-brain cannot happen. … Proven by rasterizing the SVG (resvg) and
> decoding it with a real reader (rqrr), not by string assertions.

State the problem, the approach, the trade-offs, and how you proved it works.

## Landing a pull request

`main` and the `v{major}.{minor}.x` series branches are protected. Nothing is
pushed to them directly: every pull request lands on them as a merge commit whose
message is the pull request's title and description, once its required checks
pass. No approval is required, so the release bot's own pull requests merge
themselves when their checks pass.

The `version-integrity` check reads the title and description on every pull
request:

- **The title is a conventional commit**, with a type from the table above.
- **One release group:** the scope stays free-form. The changed paths decide the
  group, not the scope. `tooling/release-groups.toml` names the groups.
- **More than one release group:** the title is `summary: …`, with no scope and
  no `!`, and the description carries exactly one header per touched group,
  scoped by the group's name:

  ```
  summary: rename the filter API

  Why these moved together.

  feat(bedrock)!: rename Filter::eq
  refactor(outpost): follow the rename
  ```

  GitHub wraps the merge commit's body at 72 columns, so keep each header under
  that or it lands split across two lines.

- **`chore(release):` and version writes belong to the release bot.** A subject
  starting `chore(release):`, or a change to a versioned crate's
  `[package].version`, fails unless `wp-botswain[bot]` authored the pull request.

Branch commits are not checked, but they stay conventional: the release notes are
rendered from them, and one unconventional commit fails the notes.

### `Covenant: break`

Files under `covenant/fixtures/` are append-only, because changing one changes
what old data means. A pull request that modifies, deletes, or renames one fails
unless its description ends with this trailer, in a paragraph of its own:

```
Covenant: break
```

Adding a fixture needs no trailer, and neither does editing a group's own
`covenant/fixtures/<group>/README.md`, which describes the directory. A
`README.md` any deeper is data, and frozen like the rest.

## Signing

Every commit is signed. This is the repo default; do not disable it. (If you
must push while signing is unavailable, push unsigned and re-sign later rather
than committing an unsigned history you intend to keep.)

## Licensing

Waypoints is licensed under the AGPL, version 3 or any later version ([plain text](../LICENSE), [Markdown](licensing/LICENSE.md)), with an [additional permission](licensing/COPYING.exceptions) for application stores. **Outside contributions are closed for now:** they will be accepted under the [contributor license agreement](licensing/CLA.md), and the portal for signing it isn't live yet, so please open an issue instead of a pull request. Every source file carries an SPDX header, and a pull request without one fails its checks.

The [licensing overview](licensing/README.md) has the header text, what the agreement promises you, and what to do with material you do not own. [`STEWARD.md`](licensing/STEWARD.md) records who the Steward is.

## Examples

```
feat(cartograph/sync): liquid QR rendering — merged rounded edges
feat(app/editor): block context menu — delete, duplicate, convert-to
fix(outpost): bind loopback-only by default — no more firewall prompts
refactor(domain): fold shared aliases into their modules
build(dev): specta tooling for build-time typegen
chore(deps): pin turso to stable 0.7.0
feat(bedrock)!: STRICT tables by default

BREAKING CHANGE: existing non-STRICT tables must be migrated via `…`.
```
