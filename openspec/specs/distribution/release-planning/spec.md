# distribution/release-planning Specification

## Purpose

Defines where a build's version comes from and how it names the channel, how versions move across the release branches, what a series branch may contain, and the invariants the release and nightly planners must hold when deciding whether and what to publish — so that the bookkeeping a release produces cannot itself be mistaken for the product work that justifies the next release, and so that any tagged tree builds the release its tag names.

## Requirements

### Requirement: The app manifest is the sole source of a build's version

Every build's version SHALL be read from the app manifest (`src-tauri/Cargo.toml`) of the commit being built. No other file SHALL supply or override it — in particular, a flavor's Tauri config SHALL NOT declare a `version`, since a Tauri config `version` takes precedence over the manifest and would pin every branch's bundle to one value.

This governs every distribution channel, not any single packaging mechanism: the bundler pipeline's platform artifacts, the store build numbers, the identity a build reports about itself, and the Nix packages alike.

A publish MAY append prerelease and build-metadata suffixes that exist only at publish time — a nightly's counter and date stamp are assigned by the planner and appear in no manifest. Such a suffix SHALL extend the manifest's version rather than replace it, so the base version of a published artifact always equals the manifest version of the commit it was built from.

#### Scenario: Version comes from the built commit's manifest

- **WHEN** a build is produced from a commit whose app manifest declares version `X`
- **THEN** the artifact reports version `X`, optionally extended by a publish-time suffix, and no flavor config overrides it

#### Scenario: Publish-time suffix extends rather than replaces

- **WHEN** a nightly is published from a commit whose manifest declares `X`
- **THEN** its version is `X` plus the planner's prerelease counter and date stamp, and stripping that suffix yields exactly `X`

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

### Requirement: Version tracks the branch a commit sits on

Each release line carries its own version in its own branch's manifest, and the lines advance independently except where they are deliberately coupled. A version's _base_ is its `major.minor.patch`, without prerelease or build metadata:

- `main` and the current preview branch (`v{minor}.x`) SHALL always declare the same base version. A change that advances one SHALL advance the other.
- The stable branch (`v{minor}.x` for the released minor) SHALL advance independently of the preview line.
- A nightly's base SHALL equal `main`'s manifest base, so a nightly's base advances only when the preview line cuts a new version.

Until `v1`, divergence between the stable and preview lines is confined to the patch component; from `v1` it extends to minor and patch.

#### Scenario: Patch to preview only

- **WHEN** a patch lands in the preview line while stable is at `0.1.0` and preview at `0.2.0`
- **THEN** `main` and the preview branch both declare base `0.2.1`, and the stable branch still declares `0.1.0`

#### Scenario: Patch to stable only

- **WHEN** a patch lands in the stable line and does not belong in preview
- **THEN** the stable branch declares `0.1.1`, while `main` and the preview branch are unchanged at base `0.2.0`

#### Scenario: Patch to both lines

- **WHEN** a patch lands in stable and is also carried into preview
- **THEN** the stable branch declares `0.1.1` and `main` and the preview branch both declare base `0.2.1`

#### Scenario: Nightly base follows the preview line

- **WHEN** the preview line advances from `0.2.0` to `0.2.1`
- **THEN** subsequently published nightlies carry base `0.2.1`, and no nightly changes its base without such a cut

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

### Requirement: Release refs are minted so their pushes are observable

Every tag and branch the planners create SHALL raise the repository's `push` event, so that workflows triggered by a tag push run for planner-minted tags exactly as for tags pushed by a person.

#### Scenario: A planner-minted tag triggers its workflows

- **WHEN** the nightly planner or the release planner creates a `v*` tag
- **THEN** every workflow triggered by a `v*` tag push runs for it

### Requirement: A nightly publishes only when product work landed

The nightly planner SHALL publish a nightly only when product work landed since the previously published nightly of the same target. A repository in which nothing landed SHALL produce no nightly tag, no build, and no publish.

#### Scenario: Quiet repository

- **WHEN** the nightly planner runs and no product work landed since the last published nightly
- **THEN** it decides to skip: no tag is created and nothing is built or published

#### Scenario: Work landed

- **WHEN** the nightly planner runs and product work landed since the last published nightly
- **THEN** it publishes the next nightly of that target, stamped with the run's date

### Requirement: Release-bookkeeping commits are not product work

Commits that touch only the release index SHALL NOT count as product work in any planner decision. This holds for both planners:

- the nightly planner's "did anything land?" determination, which is also its version bump; and
- the release planner's determination of the target a cut is heading toward, which reads the same commit history.

This requirement exists because the index necessarily lands on `main` after the tag it describes. Without it, each index commit would supply the justification for the next nightly, the quiet-repository skip would never trigger again, and a nightly would publish every day whether or not the product changed.

#### Scenario: Index commit does not wake a quiet repository

- **WHEN** the only commit on `main` since the last published nightly is the index commit recording that nightly
- **THEN** the nightly planner still decides to skip

#### Scenario: Index commit does not move a cut target

- **WHEN** the commit history since the last stable release contains index commits alongside product commits
- **THEN** the cut target is the same as it would be with the index commits absent

#### Scenario: Mixed history still publishes

- **WHEN** the history since the last published nightly contains both an index commit and at least one product commit
- **THEN** a nightly publishes, exactly as if the index commit were not there

### Requirement: The published version is unrecoverable unless recorded

A nightly's version SHALL be considered recoverable only from its tag. The planners SHALL NOT be relied upon to reproduce a previously published version: a nightly's date stamp is taken from the wall clock at plan time, so a later run of the same planner against the same commit produces a different version.

Consequently, any consumer that needs a published version — including Nix builds — SHALL read it from the recorded tag rather than recompute it.

#### Scenario: Replanning does not reproduce a published nightly

- **WHEN** the nightly planner is run on a later date against the same commit that a previous nightly was published from
- **THEN** the version it computes differs in its date stamp from the published one, and the published version is obtainable only from the tag

### Requirement: A channel's release is built and tested under its own flavor

Every release on every channel SHALL be a fresh build under that channel's flavor; a promotion SHALL rebuild under the stable flavor rather than relabel the preview build. The release test suite SHALL run under the same flavor the build enables, so the compiled combination that ships is the one that was tested. A shipped flavor MAY gate behaviour at compile time; what distinguishes channels is for each channel's own build to carry.

#### Scenario: Promotion rebuilds

- **WHEN** `v0.2.x` is promoted from `0.2.0-pre.7` to `0.2.0`
- **THEN** the stable artifacts come from a build under the stable flavor, after the release suite has run under that flavor, and no preview artifact is relabelled

#### Scenario: The suite runs under the shipped flavor

- **WHEN** the nightly release suite gates the nightly builds
- **THEN** it ran with the nightly flavor enabled on every crate that declares one, and a nightly-only code path is exercised by it

#### Scenario: A pull request is tested under its branch's channel

- **WHEN** a pull request targets `main`, whose manifest declares a nightly version
- **THEN** its test suite runs with the nightly flavor enabled; a pull request against a series branch runs with that branch's channel flavor

#### Scenario: Gates form a staircase

- **WHEN** an item is gated to the preview flavor and another to the nightly flavor
- **THEN** a nightly build compiles both, a preview build compiles the preview-gated item alone, and a stable build compiles neither

### Requirement: A build enables exactly one shipped flavor

A build SHALL enable exactly one shipped flavor, and a crate that gates on flavors SHALL refuse to compile with two shipped flavors enabled, since Cargo features are additive and flavors are not. The development flavor is not a channel: it MAY be enabled beside a shipped flavor by test and development builds, SHALL only add to a channel's behaviour and never override a channel gate, and SHALL NOT reach a released artifact.

#### Scenario: Two flavors refused

- **WHEN** feature unification would enable both `preview` and `stable` on one crate
- **THEN** the crate fails to compile, naming the conflict

#### Scenario: The wrong flavor for the tree

- **WHEN** `main` declares `0.3.0-nightly` and a build enables `stable` alone
- **THEN** the build is refused, naming `stable` and the nightly channel the manifest declares

#### Scenario: Development beside a shipped flavor

- **WHEN** `main` declares `0.3.0-nightly` and a workstation build enables `development` and `stable`
- **THEN** the build compiles stable's baseline with the development additions, and nothing treats it as a release

#### Scenario: A release build without a shipped flavor

- **WHEN** a release build enables no shipped flavor
- **THEN** the build is refused, naming the flavors it accepts

#### Scenario: A test build without a shipped flavor

- **WHEN** `cargo test` or a dev server compiles the app with no shipped flavor enabled
- **THEN** the compile succeeds on the ungated baseline, since it produces no release build

### Requirement: Every workspace crate is named in the group file

Every workspace member outside `tooling/` SHALL be named in `tooling/release-groups.toml` as a versioned member of exactly one group or an unversioned internal of exactly one group. Members under `tooling/` build the repository, not the product, and SHALL NOT be named. Each group SHALL name an anchor whose `[package].version` is the group's version. A planner SHALL refuse to run while any member outside `tooling/` is unnamed, any member is named twice, or any member under `tooling/` is named.

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
