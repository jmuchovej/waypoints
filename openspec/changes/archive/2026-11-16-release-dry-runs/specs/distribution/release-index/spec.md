# Spec Delta

## MODIFIED Requirements

### Requirement: Index entry contents

Each channel's index entry SHALL record, for the most recent release published on that channel:

- the release tag, exactly as it was created;
- the commit the tag points at;
- the source hash that pins the fetched content of that tag.

The recorded tag SHALL be the sole source of the published version, including a nightly's date stamp. An index entry SHALL NOT record a version separately from its tag, because two fields naming one value can disagree.

An entry SHALL NOT record dependency hashes. The tree it names carries its own, and is built with its own recipe.

#### Scenario: Version comes from the recorded tag

- **WHEN** an index entry records tag `v0.2.0-nightly.20260829`
- **THEN** the version attributed to that channel's release is `0.2.0-nightly.20260829`, date stamp included, regardless of the date on which the build runs

#### Scenario: Recorded hash pins the tag's source

- **WHEN** a build fetches the rev an index entry names and the fetched content does not match the entry's source hash
- **THEN** the build fails rather than proceeding with unverified content

#### Scenario: No separate version field

- **WHEN** an index entry is inspected
- **THEN** it carries no version field independent of its tag

## ADDED Requirements

### Requirement: A dry run renders the entry it would record

A dry run SHALL render the index entry its tag would receive — the would-be tag, the commit it would point at, and that commit's source hash — as an artifact of the run, applying the same channel-agreement check as a real write, and SHALL write no index file and open no pull request.

#### Scenario: Rendered, not recorded

- **WHEN** a nightly dry run completes its publishing steps
- **THEN** its artifacts hold the nightly entry as the real run would have committed it, and the index on `main` is unchanged

#### Scenario: The channel check still applies

- **WHEN** a dry run's would-be tag classifies as one channel and the commit's manifest as another
- **THEN** the render fails naming both channels, as a real write would
