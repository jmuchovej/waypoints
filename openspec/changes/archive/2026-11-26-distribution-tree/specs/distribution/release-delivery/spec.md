# Spec Delta

## MODIFIED Requirements

### Requirement: A channel's manifest is what an updater reads

Each channel SHALL expose a manifest naming that channel's current version and the targets available for it. A manifest SHALL describe only a release whose every required target was published. A manifest SHALL be served as published rather than derived on demand, and SHALL be the single answer to what version a channel is on — so that the download surface, an updater, and a package manifest can never disagree about a channel's current release.

#### Scenario: An updater learns the current version

- **WHEN** a client reads a channel's manifest
- **THEN** it names that channel's current version and the targets published for it

#### Scenario: A channel with nothing published offers no manifest

- **WHEN** a channel has never completed a publishing run
- **THEN** no manifest is available for it, and that is reported as a missing release rather than an error

#### Scenario: One answer for both readers

- **WHEN** the download surface and an updater each resolve a channel's current version
- **THEN** both read the same manifest and obtain the same version

#### Scenario: A manifest is not frozen into a build

- **WHEN** the site is deployed
- **THEN** manifests are read at request time, so a release published after that deployment is reflected without redeploying

## ADDED Requirements

### Requirement: A channel's manifest records every published file's digest and is readable whole

A channel's manifest SHALL record the SHA-256 digest of every file published for each target, and the download surface SHALL serve the manifest in full, as the publishing run wrote it, so that a package manifest derives its checksums from the record without a store credential or a re-download.

#### Scenario: The manifest itself is served

- **WHEN** a tool requests a channel's manifest from the download surface
- **THEN** it receives the stored object's bytes, digests included, without reaching the store

#### Scenario: A digest matches the stored bytes

- **WHEN** a client downloads a published file and hashes it
- **THEN** the SHA-256 equals the digest the channel's manifest records for that target and extension
