## ADDED Requirements

### Requirement: Groups own disjoint paths, and the app counts what it ships

Each group SHALL own a disjoint set of paths. A library group SHALL count commits that change its own paths; the app SHALL count commits that change its own paths or any group's. Commits under `nix/releases/` only, commits listed in `.git-blame-ignore-revs`, and the release bot's `chore(release):` commits SHALL count toward no group.

#### Scenario: Library-only change

- **WHEN** a commit changes only files under `crates/bedrock-core/`
- **THEN** it counts toward `bedrock` and `waypoints`, and not toward `outpost` or `domain`

#### Scenario: Tooling-only change

- **WHEN** a commit changes only files under `tooling/`
- **THEN** it counts toward no group

#### Scenario: Treewide formatting pass

- **WHEN** a commit is listed in `.git-blame-ignore-revs`
- **THEN** it counts toward no group

#### Scenario: Release bot bookkeeping

- **WHEN** a commit authored by the release bot carries a `chore(release):` subject
- **THEN** it counts toward no group

### Requirement: A planner reads first-parent history

A planner SHALL read `main`'s first-parent history, treating each merge commit as one commit whose changed paths are its diff against its first parent and whose message is the merge commit's own. Commits beneath a merge SHALL NOT be read.

#### Scenario: A pull request lands as one commit

- **WHEN** a pull request of three branch commits merges into `main`
- **THEN** the planners count one commit, with the merge commit's message and the diff against its first parent, and read none of the three beneath it

### Requirement: Library versions are minted only by a cut

A cut SHALL compute the next version of every library group with counted commits since its last tag, land all of them in one summary commit on `main` through a pull request, and tag each changed group at the commit as it landed. Advance, promote, and patch SHALL NOT change any library version.

#### Scenario: Cut with library work

- **WHEN** a cut runs and `bedrock` has counted commits since `bedrock-v0.3.0`
- **THEN** the summary commit sets every `bedrock` member to the next version, `bedrock-v{next}` tags that commit on `main`, and the series branch is cut from it

#### Scenario: Cut without library work

- **WHEN** a cut runs and `outpost` has no counted commits since its last tag
- **THEN** no `outpost` manifest changes and no `outpost` tag is created

#### Scenario: Tags land on the merge commit

- **WHEN** the summary pull request is merged
- **THEN** every group tag points at the merge commit on `main`, which its first-parent history contains, not at the pull request branch's commit

#### Scenario: First cut of a group with no tag

- **WHEN** a cut runs and `domain` has no `domain-v*` tag
- **THEN** the cut counts `domain`'s commits over the whole first-parent history, from the version its anchor manifest declares, as the app's first cut was measured before any stable tag existed

#### Scenario: Backport to a series branch

- **WHEN** a patch on `v0.3.x` changes files under `crates/bedrock/`
- **THEN** no library version changes

### Requirement: A cut may be scoped to one library group

A group-scoped cut SHALL mint that group alone, SHALL create no branch, SHALL NOT move the app's version or any other group's, and SHALL NOT trigger any release workflow. A library group's bump SHALL follow semantic versioning as Cargo reads it — a break bumps the minor before `1.0` and the major after — and a group-scoped cut MAY name a floor of `major`, `minor`, or `patch`, which raises the bump but never lowers it. A group-scoped cut with no counted commits SHALL refuse and name the group.

#### Scenario: Group-only cut

- **WHEN** a cut is scoped to `bedrock`, which has counted commits since `bedrock-v0.3.0`
- **THEN** one summary commit lands on `main` with a single `bedrock` header, `bedrock-v{next}` tags it, and no branch is created, no other version moves, and no release workflow runs

#### Scenario: A floor raises a group's bump

- **WHEN** a cut scoped to `outpost` at `1.4.2` finds only fixes but names a floor of `major`
- **THEN** it targets `2.0.0`; and the same floor on a cut whose commits already require a major changes nothing

#### Scenario: Nothing to cut

- **WHEN** a cut is scoped to `domain` and no counted commit has landed since `domain-v0.1.1`
- **THEN** the cut refuses and names `domain`

### Requirement: Library tags are created only on main

Library tags SHALL be created only on `main`. A series branch SHALL take a library change only by moving its pin to a tag on `main` whose line is compatible with the pin it replaces; where the group on `main` has taken a break since the series' pin, no such tag exists and the series SHALL NOT be patched for that group.

#### Scenario: A fix reaches a series by pin

- **WHEN** a `bedrock` fix has landed on `main`, `bedrock` on `main` has taken no break since `bedrock-v0.2.0`, and `v0.2.x` pins `bedrock-v0.2.0`
- **THEN** a cut scoped to `bedrock` mints `bedrock-v0.2.1` on `main`, and `patch` on `v0.2.x` moves the series' pin to it and bumps the app

#### Scenario: A line broken on main cannot be patched into a series

- **WHEN** `v0.2.x` pins `bedrock-v0.2.0` and `bedrock` on `main` has taken a break since
- **THEN** `patch` on `v0.2.x` refuses for `bedrock`, names the breaking commit, and points at a new cut as the way to ship the fix

### Requirement: A multi-group commit is a summary

A commit that changes the own paths of more than one group SHALL use the `summary` type in its subject and carry in its body one conventional header per touched group, scoped by group name. Each group's severity SHALL come from its own header, and the subject SHALL count for no group. A library group's header SHALL follow the 0.x right-shift: a feature is a patch and a break is a minor until that group reaches 1.0.

#### Scenario: Library break with caller edits elsewhere

- **WHEN** a summary carries `feat(bedrock)!: …` and `refactor(outpost): …`
- **THEN** it is breaking for `bedrock`, non-breaking for `outpost`, and a feature for `waypoints`

#### Scenario: Single-group commit with a free-form scope

- **WHEN** `fix(query): …` changes only files under `crates/bedrock/`
- **THEN** it counts as a fix for `bedrock` and for `waypoints`

### Requirement: The app breaks only on a covenant trailer

A commit SHALL be breaking for the app if and only if it carries the trailer `Covenant: break`; a `!` in any header SHALL NOT break the app. A cut SHALL target a major for the app only when invoked with `--major`; when a breaking commit exists since the last stable and `--major` is absent, the cut SHALL refuse and name the commits. A feature SHALL bump the app's minor even before 1.0.

#### Scenario: Break marker without the trailer after v1

- **WHEN** the last stable is `1.2.0` and the only commit is `feat!: …` without the trailer
- **THEN** the cut targets `1.3.0`

#### Scenario: The trailer after v1

- **WHEN** the last stable is `1.2.0` and any commit carries `Covenant: break`
- **THEN** `cut` refuses and names that commit, and `cut --major` targets `2.0.0`

#### Scenario: Product major without the trailer

- **WHEN** the last stable is `1.2.0`, no commit carries `Covenant: break`, and the cut is invoked with `--major`
- **THEN** the cut targets `2.0.0`

### Requirement: Evidence sets a floor on each group's bump

A group's bump SHALL be the greater of the bump its commits declare and the bump its evidence requires. The evidence SHALL be the API-compatibility check of each inspectable library member against the group's last tag, and any modified, deleted, or renamed file under `covenant/fixtures/<group>/` since the last stable tag, which is breaking for that group and for the app.

#### Scenario: Undeclared API break reaches a cut

- **WHEN** the API check reports a break in `outpost` against `outpost-v0.2.0` and no commit declares one
- **THEN** the cut computes a breaking bump for `outpost`

#### Scenario: Domain fixture modified

- **WHEN** a file under `covenant/fixtures/domain/` was modified since the last stable tag
- **THEN** the cut computes a breaking bump for `domain` and for `waypoints`

#### Scenario: Declared break with no evidence

- **WHEN** a commit declares `fix(bedrock)!: …` and the API check reports no break
- **THEN** the cut computes a breaking bump for `bedrock` from the declaration alone

#### Scenario: Procedural macro member

- **WHEN** `bedrock-derive`, a member whose API the checker cannot inspect, is among a cut's counted changes
- **THEN** it contributes no detected evidence, and `bedrock` is versioned on its declarations and fixtures alone

#### Scenario: Group with no tag yet

- **WHEN** a cut runs and `transload` has no `transload-v*` tag to measure against
- **THEN** the API check is skipped for `transload`, the cut says so naming the group, and the bump comes from declarations and the fixture floor alone

### Requirement: The API floor measures the stable surface

The API check SHALL measure the surface compiled under the stable flavor, since that is the surface every series consumes. A change gated to the nightly or preview flavor SHALL NOT count as detected evidence.

#### Scenario: A nightly-gated break is not evidence

- **WHEN** a `bedrock` commit removes a public item that exists only under `cfg(feature = "nightly")`
- **THEN** the API check under the stable flavor reports no break, and the commit's own header decides the bump

### Requirement: Builds embed every group's version

Every build SHALL embed each group's version alongside the app's build information. On `main` it is the group's latest tag as described from the tree; on a series branch it is the tag the branch pins. Where the source carries neither, the build SHALL take injected values instead. Only workspace groups SHALL be embedded.

#### Scenario: Built at a group tag

- **WHEN** a build is produced on `main` from the commit tagged `bedrock-v0.4.0`
- **THEN** it reports `bedrock` as `bedrock-v0.4.0`

#### Scenario: Built on a series

- **WHEN** a build is produced on `v0.4.x`, which pins `bedrock-v0.4.0`
- **THEN** it reports `bedrock` as exactly `bedrock-v0.4.0`, read from the pin, whatever the branch's in-tree copy contains

#### Scenario: Source without tags

- **WHEN** a build runs from a shallow checkout or a Nix source with injected group versions
- **THEN** it reports the injected values

### Requirement: Each channel builds the library versions its branch pins

A library group's released versions SHALL be addressable by tag, `<group>-v{version}`, distinct from the app's bare `v{version}`. A series branch SHALL build each library group at a tag it pins, and MAY pin a different tag per group than any other series branch pins; `main` SHALL build the in-tree libraries, so the nightly channel is where library work meets the app first. A cut SHALL pin the series it opens to the tags it minted.

#### Scenario: Two series on different library versions

- **WHEN** `v0.3.x` pins `bedrock-v0.2.0` and `v0.2.x` still pins `bedrock-v0.1.0`
- **THEN** each series builds the bedrock its own pin names, and neither is affected by `main`'s in-tree bedrock

#### Scenario: Main builds in-tree

- **WHEN** a nightly is built from `main` while `crates/bedrock` declares `0.3.0`
- **THEN** the build uses the in-tree `crates/bedrock`, whatever tag any series pins

#### Scenario: A cut starts pinned to what it minted

- **WHEN** a cut mints `bedrock-v0.2.0` and `domain-v0.1.1`
- **THEN** the series branch it opens pins exactly those tags, and no other group's pin moves

#### Scenario: One copy of each group per binary

- **WHEN** a series builds the app
- **THEN** every crate the app ships resolves each group from the same tag, so the binary carries one copy of each group
