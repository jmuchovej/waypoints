# Spec Delta

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
