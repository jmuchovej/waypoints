# distribution/release-index Specification

## Purpose
Defines the committed record of what Waypoints most recently published on each release channel — the tag, the commit, and the hash needed to fetch that release's source — so that a build from `main` at any later time reconstructs a published release rather than whatever the current tree happens to contain.

## Requirements

### Requirement: Per-channel index files

The repository SHALL carry one release-index file per release channel — stable, preview, and nightly — under a single dedicated directory on the `main` branch. Series branches SHALL NOT carry an index; a series branch's own tree is its release.

Channels SHALL NOT share a file. Each channel's file SHALL be independently writable, so that two channels publishing concurrently produce no textual conflict between their index writes. The set of files SHALL NOT grow with releases: each file records the latest release only.

#### Scenario: Three channels, three files

- **WHEN** the release index is inspected on `main`
- **THEN** exactly one file exists for each of stable, preview, and nightly, each under the same dedicated index directory

#### Scenario: Concurrent publishes on different channels

- **WHEN** a stable release and a nightly release are published close enough together that both index writes are in flight
- **THEN** each write touches only its own channel's file and neither write conflicts with or overwrites the other

#### Scenario: Series branches carry no index

- **WHEN** a series branch (`v0.1.x`, `v0.2.x`) is inspected
- **THEN** its index directory is whatever it was when the branch was cut and is not consulted; the branch builds its own tree

### Requirement: Index entry contents

Each channel's index entry SHALL record, for the most recent release published on that channel:

- the release tag, exactly as it was created;
- the commit the tag points at;
- the source hash that pins the fetched content of that tag.

The recorded tag SHALL be the sole source of the published version, including a nightly's build-metadata date stamp. An index entry SHALL NOT record a version separately from its tag, because two fields naming one value can disagree.

An entry SHALL NOT record dependency hashes. The tree it names carries its own, and is built with its own recipe.

#### Scenario: Version comes from the recorded tag

- **WHEN** an index entry records tag `v0.2.0-nightly.7+2026.08.29`
- **THEN** the version attributed to that channel's release is `0.2.0-nightly.7+2026.08.29`, date stamp included, regardless of the date on which the build runs

#### Scenario: Recorded hash pins the tag's source

- **WHEN** a build fetches the rev an index entry names and the fetched content does not match the entry's source hash
- **THEN** the build fails rather than proceeding with unverified content

#### Scenario: No separate version field

- **WHEN** an index entry is inspected
- **THEN** it carries no version field independent of its tag

### Requirement: The index is metadata, not code

An index file SHALL contain only the record of a published release. It SHALL NOT influence, parameterize, or participate in what any build produces other than by selecting and verifying the source that build fetches. The recipe that builds a recorded release is the recorded tree's own.

Because the index is metadata, an index on `main` SHALL be able to describe a tag that exists only on a series branch, with no branch-specific handling — tags are repository-global.

#### Scenario: Index on main describes a series-branch tag

- **WHEN** the stable index entry names a tag that was created on the `v0.1.x` branch
- **THEN** the stable package resolves and builds that tag from the index on `main`, with that tag's own recipe and no reference to `v0.1.x` as a branch

#### Scenario: Index change alters no build output

- **WHEN** an index file is edited in a way that leaves its tag, rev, and hash unchanged (formatting, key order)
- **THEN** every package's output is unchanged

### Requirement: The index is written after its tag exists

A channel's index entry SHALL be written only after the tag it describes has been created. An index entry SHALL NOT be written in the same commit that the tag points at, and SHALL NOT be a precondition of tagging.

This ordering is forced, not chosen: the entry records the tag's rev and source hash, so an entry written before tagging would have to contain a hash of content that includes itself.

The writer SHALL refuse a tag whose version classifies as one channel while the tagged tree's manifest version classifies as another.

#### Scenario: Tag precedes its index entry

- **WHEN** a release is published on any channel
- **THEN** the tag is created first, and the index entry naming that tag is committed afterwards, to `main`

#### Scenario: A failed index write leaves a valid release

- **WHEN** a tag is created successfully but the subsequent index write fails
- **THEN** the release itself remains valid and published, and the channel's package continues to resolve its previous entry until the index is written

#### Scenario: A wrongly minted tag is not recorded

- **WHEN** a tag whose version reads as a preview points at a tree whose manifest version reads as nightly
- **THEN** the writer fails without touching any index file, naming both channels

### Requirement: A channel with no entry does not resolve

Until a channel has published at least one release recorded in the index, that channel SHALL have no index entry, and its package on `main` SHALL fail to evaluate with a message naming the missing entry. A package SHALL NOT fall back to building the current tree, an adjacent channel's entry, or a guessed version.

#### Scenario: Pre-index channel

- **WHEN** a consumer evaluates a package on `main` whose channel has no index entry
- **THEN** evaluation fails with an error identifying the channel and its absent entry

#### Scenario: No silent fallback

- **WHEN** a channel's index entry is absent or unreadable
- **THEN** no build is produced under that channel's identity by any other means
