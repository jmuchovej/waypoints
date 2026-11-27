## ADDED Requirements

### Requirement: The group file declares a build's version

Every build's version SHALL be the version `tooling/release-groups.toml` declares for the app at the commit being built, which the app manifest SHALL project exactly; a build MAY read either, since a landed tree holds them equal. No other file SHALL supply or override it, and a flavor's Tauri config SHALL NOT declare a `version`. A publish-time prerelease suffix SHALL extend the declared version rather than replace it.

#### Scenario: Version comes from the built commit's declaration

- **WHEN** a build is produced from a commit whose group file declares app version `X`, projected into its app manifest
- **THEN** the artifact reports version `X`, optionally extended by a publish-time suffix, and no flavor config overrides it

#### Scenario: Publish-time suffix extends rather than replaces

- **WHEN** a nightly is published from a commit whose group file declares `X`
- **THEN** its version is `X` plus the planner's date-stamped prerelease identifier, and stripping that suffix yields exactly `X`

#### Scenario: Store build numbers and Nix packages read the same declaration

- **WHEN** a store build number, the identity a build reports about itself, or a Nix package needs the version of a commit
- **THEN** each reads the group file's app version or its projection, and all of them agree

### Requirement: A group's current version is the group file's declaration

A planner SHALL take each group's current version — every library group's and the app's — from the group file's `version` key, not from any manifest. A build that embeds group versions SHALL, where it has no injected value and no tag to describe, report the group file's declared version.

#### Scenario: A cut reads the current version from the file

- **WHEN** a cut plans `bedrock` while the group file declares `0.3.0` for it
- **THEN** the plan's current version for `bedrock` is `0.3.0`, and its next version is computed from that

#### Scenario: A series transition reads the app's version from the file

- **WHEN** `advance` plans on a series whose group file declares `0.2.0-pre.1` for the app
- **THEN** the plan targets `0.2.0-pre.2`

#### Scenario: A shallow build falls back to the declaration

- **WHEN** a build runs from a shallow checkout with no injected group versions, no pin in its lockfile, and no tag to describe
- **THEN** it reports each library group at the version the group file declares

### Requirement: A version write lands the group file and its projections together

Every version write SHALL set a group's `version` in the group file and every member manifest's `[package].version` and lockfile entry in one commit. A cut SHALL write the file, every changed member manifest, and the lockfile in its one summary commit on `main`. Advance, promote, and patch SHALL write the app's version to the file and the app's manifests, and no library group's; on a series the file's library versions stay what the cut minted, which is the series' bill of materials.

#### Scenario: A cut writes the file with the manifests

- **WHEN** a cut mints `bedrock-v0.2.0` and `domain-v0.1.1` and moves `main` to `0.3.0-nightly`
- **THEN** the summary commit sets `bedrock` to `0.2.0`, `domain` to `0.1.1`, and the app to `0.3.0-nightly` in the group file, and every member manifest and lockfile entry of those groups to the same

#### Scenario: An advance writes the app alone

- **WHEN** `advance` runs on `v0.2.x` from `0.2.0-pre.1`
- **THEN** the commit sets the app to `0.2.0-pre.2` in the group file and in `src-tauri` and `crates/cartograph`, and no library group's version changes in the file

#### Scenario: A patch writes the app alone

- **WHEN** `patch` runs on `v0.2.x` from `0.2.0`, after a `bedrock` fix was cherry-picked onto the series
- **THEN** the commit sets the app to `0.2.1` in the group file and the app's manifests, and `bedrock`'s version in the series' group file is unchanged

### Requirement: A command projects the group file into the member manifests

A command SHALL write every group's declared version into its member manifests and their lockfile entries, from the group file as it stands, so a hand-edited file or a crate newly joining a group leaves no manifest disagreeing with it. On a tree whose manifests already agree it SHALL change nothing. The release bot's writes SHALL use the same projection.

#### Scenario: A hand-edited file is projected

- **WHEN** the group file's `domain` version is edited to `0.2.0` and the command runs
- **THEN** `crates/domain/Cargo.toml` and its lockfile entry declare `0.2.0`

#### Scenario: A new member takes its group's version

- **WHEN** a crate is added to `bedrock`'s members with its manifest at `0.0.0` and the command runs
- **THEN** that manifest declares `bedrock`'s version from the file

#### Scenario: An agreeing tree is untouched

- **WHEN** the command runs on a tree whose every member manifest equals its group's declared version
- **THEN** no file changes

### Requirement: A dry run renders the group file's write

A release-cycle transition run dry SHALL render the group file's version write alongside the manifest writes it already renders: in the `main`-side patch, each minted group's version and the app's next `main` version; in the series patch, the app's version. The working tree SHALL stay unchanged.

#### Scenario: A dry cut renders the file

- **WHEN** `cut` runs dry and would mint `bedrock-v0.2.0` and move `main` to `0.3.0-nightly`
- **THEN** the `main`-side patch carries the group file's hunk setting `bedrock` to `0.2.0` and the app to `0.3.0-nightly`, beside every `bedrock` member's manifest and the app's

#### Scenario: A dry series transition renders the app's line

- **WHEN** `patch` runs dry on `v0.2.x` from `0.2.0`
- **THEN** the series patch carries the group file's hunk setting the app to `0.2.1` beside the app's manifests, and no library group's line

## MODIFIED Requirements

### Requirement: The manifest version names the channel

Every tree's channel SHALL be readable from its app's version alone — the version the group file declares for the app, which the app manifest projects: a version whose prerelease begins with `nightly` is the nightly line, a version with any other prerelease is a preview, and a version with no prerelease is stable. `main` SHALL declare `X.Y.Z-nightly`. A series branch SHALL declare `X.Y.Z-pre.N` from the commit that cuts it and a bare `X.Y.Z` from the commit that promotes it. No other file SHALL declare a channel.

A versioned member of a release group other than the app SHALL mirror its group's declared version exactly, prerelease included, and SHALL NOT be read as a version or channel source by anything. Mirroring the app's prerelease is therefore not a channel declaration: the app's declared version, and the app manifest that projects it, remain the only sources any build, planner, or package consults.

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

- **WHEN** `main`'s group file declares `0.3.0-nightly` for the app and `crates/cartograph` mirrors it
- **THEN** the tree's channel is still read from the app's version alone, and `cartograph`'s version declares nothing

### Requirement: Every workspace crate is named in the group file

Every workspace member outside `tooling/` SHALL be named in `tooling/release-groups.toml` as a versioned member of exactly one group or an unversioned internal of exactly one group. Members under `tooling/` build the repository, not the product, and SHALL NOT be named. Each group SHALL declare its `version`, which every versioned member's `[package].version` projects. A planner SHALL refuse to run while any member outside `tooling/` is unnamed, any member is named twice, any member under `tooling/` is named, or any group declares no version.

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

#### Scenario: Group without a version

- **WHEN** a group in the file declares no `version`, or one that is not a semantic version
- **THEN** the planner refuses to run and names the group

## REMOVED Requirements

### Requirement: The app manifest is the sole source of a build's version

**Reason**: The app manifest is no longer the declaration of a build's version but its projection; the group file declares it, and the app manifest is held equal to it. The requirement's substance — one source, no flavor-config override, a publish-time suffix that extends rather than replaces — continues under _The group file declares a build's version_, which carries both of its scenarios.

**Migration**: Readers of the app manifest's version keep working, since a landed tree holds the manifest equal to the file; readers that want the declaration itself read `tooling/release-groups.toml`'s app `version`.
