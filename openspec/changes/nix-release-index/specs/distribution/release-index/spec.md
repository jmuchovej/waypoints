## Purpose

Defines the committed record of what Waypoints actually published on each release channel — the tag, the commit, and the hashes needed to reproduce that release's source — so that a build performed at any later time can reconstruct a published release rather than rebuild whatever the current tree happens to contain.

## ADDED Requirements

### Requirement: Per-channel index files

The repository SHALL carry one release-index file per release channel — stable, preview, and nightly — under a single dedicated directory on the `main` branch. Release branches SHALL NOT carry an index.

Channels SHALL NOT share a file. Each channel's file SHALL be independently writable, so that two channels publishing concurrently produce no textual conflict between their index writes.

#### Scenario: Three channels, three files

- **WHEN** the release index is inspected on `main`
- **THEN** exactly one file exists for each of stable, preview, and nightly, each under the same dedicated index directory

#### Scenario: Concurrent publishes on different channels

- **WHEN** a stable release and a nightly release are published close enough together that both index writes are in flight
- **THEN** each write touches only its own channel's file and neither write conflicts with or overwrites the other

#### Scenario: Release branches carry no index

- **WHEN** a release branch (`v0.1.x`, `v0.2.x`) is inspected
- **THEN** no release-index file is present on it

### Requirement: Index entry contents

Each channel's index entry SHALL record, for the most recent release published on that channel:

- the release tag, exactly as it was created;
- the commit the tag points at;
- the source hash that pins the fetched content of that tag;
- the frontend dependency (pnpm fixed-output) hash for that source.

The recorded tag SHALL be the sole source of the published version, including a nightly's build-metadata date stamp. An index entry SHALL NOT record a version separately from its tag, because two fields naming one value can disagree.

The dependency hash SHALL be recorded rather than derived. Deriving it is not possible: the fetched source becomes available only at build time, while the frontend dependency fetch requires its hash at evaluation time.

#### Scenario: Version comes from the recorded tag

- **WHEN** an index entry records tag `v0.2.0-nightly.7+2026.08.29`
- **THEN** the version attributed to that channel's release is `0.2.0-nightly.7+2026.08.29`, date stamp included, regardless of the date on which the build runs

#### Scenario: Recorded hashes pin the tag's source

- **WHEN** a build fetches the rev an index entry names and the fetched content does not match the entry's source hash
- **THEN** the build fails rather than proceeding with unverified content

#### Scenario: No separate version field

- **WHEN** an index entry is inspected
- **THEN** it carries no version field independent of its tag

### Requirement: The index is metadata, not code

An index file SHALL contain only the record of a published release. It SHALL NOT influence, parameterize, or participate in what any build produces other than by selecting and verifying the source that build fetches.

Because the index is metadata, an index on `main` SHALL be able to describe a tag that exists only on a release branch, with no branch-specific handling — tags are repository-global.

#### Scenario: Index on main describes a release-branch tag

- **WHEN** the stable index entry names a tag that was created on the `v0.1.x` branch
- **THEN** the stable package resolves and builds that tag from the index on `main`, with no reference to `v0.1.x` as a branch

#### Scenario: Index change alters no build output

- **WHEN** an index file is edited in a way that leaves its tag, rev, and hashes unchanged (formatting, key order, added comments)
- **THEN** every package's output is unchanged

### Requirement: The index is written after its tag exists

A channel's index entry SHALL be written only after the tag it describes has been created. An index entry SHALL NOT be written in the same commit that the tag points at, and SHALL NOT be a precondition of tagging.

This ordering is forced, not chosen: the entry records the tag's rev and source hash, so an entry written before tagging would have to contain a hash of content that includes itself.

#### Scenario: Tag precedes its index entry

- **WHEN** a release is published on any channel
- **THEN** the tag is created first, and the index entry naming that tag is committed afterwards, to `main`

#### Scenario: A failed index write leaves a valid release

- **WHEN** a tag is created successfully but the subsequent index write fails
- **THEN** the release itself remains valid and published, and the channel's package continues to resolve its previous entry until the index is written

### Requirement: A channel with no entry does not resolve

Until a channel has published at least one release recorded in the index, that channel SHALL have no index entry, and its package SHALL fail to evaluate with a message naming the missing entry. A package SHALL NOT fall back to building the current tree, an adjacent channel's entry, or a guessed version.

#### Scenario: Pre-index channel

- **WHEN** a consumer evaluates a package whose channel has no index entry
- **THEN** evaluation fails with an error identifying the channel and its absent entry

#### Scenario: No silent fallback

- **WHEN** a channel's index entry is absent or unreadable
- **THEN** no build is produced under that channel's identity by any other means
