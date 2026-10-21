## ADDED Requirements

### Requirement: Every workspace crate is named in the group file

Every workspace member outside `tooling/` SHALL be named in `tooling/release-groups.toml` as a versioned member of exactly one group or an unversioned internal of exactly one group. Members under `tooling/` build the repository rather than the product and SHALL NOT be named. Each group SHALL name an anchor whose `[package].version` is the group's version. A planner SHALL refuse to run while any member outside `tooling/` is unnamed, any member is named twice, or any member under `tooling/` is named, and SHALL name the offending crate.

#### Scenario: Unnamed crate

- **WHEN** `crates/importer` is a workspace member and the file does not name it
- **THEN** the planner refuses to run and names the crate

#### Scenario: Crate in two groups

- **WHEN** one crate is named by two groups
- **THEN** the planner refuses to run and names both groups

#### Scenario: Tooling crates are out of scope

- **WHEN** `tooling/xtask` and `tooling/gh-workflow` are workspace members and the file names neither
- **THEN** the planner runs; and if the file did name one, it would refuse and name the crate

#### Scenario: New crate joins the workspace

- **WHEN** a crate such as `crates/transload` becomes a workspace member
- **THEN** every planner run refuses until the file names it

### Requirement: Groups own disjoint paths, and the app counts what it ships

Each group SHALL own a disjoint set of paths. A library group SHALL count commits that change its own paths; the app SHALL count commits that change its own paths or any group's. Commits under `nix/releases/` only, commits listed in `.git-blame-ignore-revs`, and the release bot's `chore(release):` commits SHALL count toward no group.

A planner SHALL read `main`'s first-parent history, treating each merge commit as one commit whose changed paths are its diff against its first parent and whose message is the merge commit's own. Commits beneath a merge SHALL NOT be read.

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

#### Scenario: A pull request lands as one commit

- **WHEN** a pull request of three branch commits merges into `main`
- **THEN** the planners count one commit, with the merge commit's message and the diff against its first parent, and read none of the three beneath it

### Requirement: Library versions are minted only by a cut

A cut SHALL compute the next version of every library group with counted commits since its last tag, land all of them in one summary commit on `main` through a pull request, and tag each changed group at the commit as it landed. Advance, promote, and patch SHALL NOT change any library version.

A cut MAY be scoped to one library group. A group-scoped cut SHALL mint that group alone, SHALL create no branch, SHALL NOT move the app's version or any other group's, and SHALL NOT trigger any release workflow. A library group's bump SHALL follow semantic versioning as Cargo reads it — a break bumps the minor before `1.0` and the major after — and a group-scoped cut MAY name a floor of `major`, `minor`, or `patch`, which raises the bump but never lowers it. A group-scoped cut with no counted commits SHALL refuse and name the group.

#### Scenario: Group-only cut

- **WHEN** a cut is scoped to `bedrock`, which has counted commits since `bedrock-v0.3.0`
- **THEN** one summary commit lands on `main` with a single `bedrock` header, `bedrock-v{next}` tags it, and no branch is created, no other version moves, and no release workflow runs

#### Scenario: A floor raises a group's bump

- **WHEN** a cut scoped to `outpost` at `1.4.2` finds only fixes but names a floor of `major`
- **THEN** it targets `2.0.0`; and the same floor on a cut whose commits already require a major changes nothing

#### Scenario: Nothing to cut

- **WHEN** a cut is scoped to `domain` and no counted commit has landed since `domain-v0.1.1`
- **THEN** the cut refuses and names `domain`

#### Scenario: Cut with library work

- **WHEN** a cut runs and `bedrock` has counted commits since `bedrock-v0.3.0`
- **THEN** the summary commit sets every `bedrock` member to the next version, `bedrock-v{next}` tags that commit on `main`, and the series branch is cut from it

#### Scenario: Cut without library work

- **WHEN** a cut runs and `outpost` has no counted commits since its last tag
- **THEN** no `outpost` manifest changes and no `outpost` tag is created

#### Scenario: Tags land on the merge commit

- **WHEN** the summary pull request is merged
- **THEN** every group tag points at the merge commit on `main`, which its first-parent history contains, not at the pull request branch's commit

#### Scenario: Backport to a series branch

- **WHEN** a patch on `v0.3.x` changes files under `crates/bedrock/`
- **THEN** no library version changes

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

A group's bump SHALL be the greater of the bump its commits declare and the bump its evidence requires. The evidence SHALL be the API-compatibility check of each inspectable library member against the group's last tag, and any modified, deleted, or renamed file under `covenant/fixtures/<group>/` since the last stable tag, which is breaking for that group and for the app. A member whose API the checker cannot inspect — a procedural macro — SHALL contribute no detected evidence and SHALL be versioned on its declarations alone.

#### Scenario: Undeclared API break reaches a cut

- **WHEN** the API check reports a break in `outpost` against `outpost-v0.2.0` and no commit declares one
- **THEN** the cut computes a breaking bump for `outpost`

#### Scenario: Domain fixture modified

- **WHEN** a file under `covenant/fixtures/domain/` was modified since the last stable tag
- **THEN** the cut computes a breaking bump for `domain` and for `waypoints`

#### Scenario: Declared break with no evidence

- **WHEN** a commit declares `fix(bedrock)!: …` and the API check reports no break
- **THEN** the cut computes a breaking bump for `bedrock` from the declaration alone

### Requirement: Builds embed every group's version

Every build SHALL embed each group's version, as described by that group's latest tag, alongside the app's build information. Where the source carries no tags, the build SHALL take injected values instead. Only workspace groups SHALL be embedded.

#### Scenario: Built at a group tag

- **WHEN** a build is produced from the commit tagged `bedrock-v0.4.0`
- **THEN** it reports `bedrock` as `bedrock-v0.4.0`

#### Scenario: Built from a backport

- **WHEN** a build is produced two commits past `bedrock-v0.4.0` on `v0.4.x`
- **THEN** it reports `bedrock` as `bedrock-v0.4.0-2-g<sha>`

#### Scenario: Source without tags

- **WHEN** a build runs from a shallow checkout or a Nix source with injected group versions
- **THEN** it reports the injected values

### Requirement: Each channel builds the library versions its branch pins

A library group's released versions SHALL be addressable by tag, `<group>-v{version}`, distinct from the app's bare `v{version}`. A series branch SHALL build each library group at a tag it pins, and MAY pin a different tag per group than any other series branch pins; `main` SHALL build the in-tree libraries, so the nightly channel is where library work meets the app first. A cut SHALL pin the series it opens to the tags it minted. Every crate the app ships SHALL pin the same tag per group, so one binary carries one copy of each group.

#### Scenario: Two series on different library versions

- **WHEN** `v0.3.x` pins `bedrock-v0.2.0` and `v0.2.x` still pins `bedrock-v0.1.0`
- **THEN** each series builds the bedrock its own pin names, and neither is affected by `main`'s in-tree bedrock

#### Scenario: Main builds in-tree

- **WHEN** a nightly is built from `main` while `crates/bedrock` declares `0.3.0`
- **THEN** the build uses the in-tree `crates/bedrock`, whatever tag any series pins

#### Scenario: A cut starts pinned to what it minted

- **WHEN** a cut mints `bedrock-v0.2.0` and `domain-v0.1.1`
- **THEN** the series branch it opens pins exactly those tags, and no other group's pin moves

### Requirement: A channel's release is built and tested under its own flavor

Every release on every channel SHALL be a fresh build under that channel's flavor; a promotion SHALL rebuild under the stable flavor rather than relabel the preview build. The release test suite SHALL run under the same flavor the build enables, so the compiled combination that ships is the one that was tested. A build SHALL enable exactly one shipped flavor, and a crate that gates on flavors SHALL refuse to compile with two shipped flavors enabled, since Cargo features are additive and flavors are not. The development flavor is not a channel: it MAY be enabled beside a shipped flavor by test and development builds, SHALL only add to a channel's behaviour and never override a channel gate, and SHALL NOT reach a released artifact. A shipped flavor MAY gate behaviour at compile time; what distinguishes channels is for each channel's own build to carry.

#### Scenario: Promotion rebuilds

- **WHEN** `v0.2.x` is promoted from `0.2.0-pre.7` to `0.2.0`
- **THEN** the stable artifacts come from a build under the stable flavor, after the release suite has run under that flavor, and no preview artifact is relabelled

#### Scenario: The suite runs under the shipped flavor

- **WHEN** the nightly release suite gates the nightly builds
- **THEN** it ran with the nightly flavor enabled on every crate that declares one, and a nightly-only code path is exercised by it

#### Scenario: Two flavors refused

- **WHEN** feature unification would enable both `preview` and `stable` on one crate
- **THEN** the crate fails to compile, naming the conflict

## MODIFIED Requirements

### Requirement: The manifest version names the channel

Every tree's channel SHALL be readable from its app manifest's version alone: a version whose prerelease begins with `nightly` is the nightly line, a version with any other prerelease is a preview, and a version with no prerelease is stable. `main` SHALL declare `X.Y.Z-nightly`. A series branch SHALL declare `X.Y.Z-pre.N` from the commit that cuts it and a bare `X.Y.Z` from the commit that promotes it. No other file SHALL declare a channel.

A versioned member of a release group other than the app SHALL mirror its group's anchor version exactly, prerelease included, and SHALL NOT be read as a version or channel source by anything. Mirroring the app's prerelease is therefore not a channel declaration: the app manifest remains the only manifest any build, planner, or package consults.

Because a tag is minted from the manifest it points at, a tag's version and its tree's manifest version SHALL classify as the same channel.

#### Scenario: Main is the nightly line

- **WHEN** `main` is inspected at any commit
- **THEN** its manifest version carries the `nightly` prerelease

#### Scenario: A cut declares preview

- **WHEN** the planner cuts `v0.2.x` from `main`
- **THEN** the branch's manifest declares `0.2.0-pre.1`, the tag `v0.2.0-pre.1` points at that tree, and `main` moves on to `0.3.0-nightly`

#### Scenario: A promotion declares stable

- **WHEN** the planner promotes `v0.2.x`
- **THEN** the branch's manifest declares `0.2.0` and the tag `v0.2.0` points at that tree

#### Scenario: A tag and its tree cannot disagree

- **WHEN** a tag's version classifies as one channel and the tagged tree's manifest version classifies as another
- **THEN** the release index refuses to record it

#### Scenario: A lockstep member mirrors the app's prerelease

- **WHEN** `main`'s app manifest declares `0.3.0-nightly` and `crates/cartograph` mirrors it
- **THEN** the tree's channel is still read from the app manifest alone, and `cartograph`'s version declares nothing

### Requirement: A series branch carries only cherry-picks of `main`

A series branch SHALL be cut from `main` and SHALL thereafter receive only the planner's version commits and cherry-picks of commits already on `main`, in the order those commits landed on `main`. No change SHALL land on a series branch first. A series branch's tree therefore always carries a recipe, lockfiles, and pins that some commit of `main` was built with, which is what lets a tagged tree build itself.

Where the branch is protected, the planner's version commit SHALL arrive as the merge commit of a pull request the release bot opened, gated by the same required checks as any other pull request. Such a merge commit SHALL carry only that version write, and SHALL NOT introduce any change that is not already on `main`, so the invariant above is unchanged by how the commit lands.

#### Scenario: A fix reaches a series

- **WHEN** a fix merged on `main` is needed on `v0.2.x`
- **THEN** it reaches the branch as a cherry-pick of the `main` commit, after any earlier `main` commits it depends on have been cherry-picked

#### Scenario: A tagged tree is self-sufficient

- **WHEN** any `v*` tag is checked out and built under the channel its manifest version names
- **THEN** the build succeeds using only that tree's recipe, lockfiles, and pins

#### Scenario: A version commit lands on a protected series branch

- **WHEN** the planner advances `v0.2.x` and the branch is protected
- **THEN** the version write lands as the merge commit of the release bot's pull request, carries no other change, and the tag points at that merge commit
