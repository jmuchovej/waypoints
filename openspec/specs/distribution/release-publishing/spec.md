# distribution/release-publishing Specification

## Purpose
Governs what a published release is and how one comes to exist: which channels elect to publish and on what evidence, where a release's bytes are stored and under what name, the per-channel manifest that is the single point at which a release becomes advertised, the rule that every target for a channel must succeed before anything is advertised, what a run withdraws when it falls short, and the integrity standing of the credential that writes to the store.

## Requirements

### Requirement: A release's bytes live in the artifact store, its record in the forge

A published release SHALL consist of two parts held in two places. Built artifacts SHALL be written to the artifact store, which is private and reachable only under a bucket-scoped credential. The release record — the tag and its generated notes — SHALL be held by the forge. A release record SHALL NOT carry artifact bytes, and the artifact store SHALL NOT be the source of release notes.

#### Scenario: Artifacts reach only the store

- **WHEN** a release build produces a distributable bundle
- **THEN** that bundle is uploaded to the artifact store and is not attached to the forge's release record

#### Scenario: Notes reach only the record

- **WHEN** a release's notes are generated for a tag
- **THEN** they are written to the forge's release record and no copy is placed in the artifact store

### Requirement: An artifact's key is derived from its release, and is written once

Each artifact SHALL be stored under a key derived from its channel, version, operating system, and architecture, grouped so that one prefix holds a whole channel and a nested prefix holds a whole minor line, and so that no two distinct builds share a key. The channel in a key SHALL be derived from the version rather than supplied independently, and a reader SHALL recompute it and refuse any request whose channel disagrees with its version. A key SHALL be written at most once: an upload SHALL NOT overwrite an existing object. Where a version carries a build counter, that counter SHALL form part of the key, so that a rebuild of an already-published version occupies a new key rather than replacing the bytes of the old one.

#### Scenario: The channel in a key is derived, not supplied

- **WHEN** an artifact's storage location is determined
- **THEN** its channel comes from its version's grammar rather than from any independent input, so an artifact cannot be filed under a channel its version does not belong to

#### Scenario: One prefix selects a whole channel

- **WHEN** a retention policy is expressed for a single channel
- **THEN** one key prefix selects every artifact of that channel and no artifact of another, across every minor line

#### Scenario: A target may carry more than one artifact

- **WHEN** a target produces more than one distributable form of the same version
- **THEN** each occupies its own key under that target's prefix, and the target counts once toward completeness

#### Scenario: Two platforms of one version do not collide

- **WHEN** two platforms upload their artifacts for the same version
- **THEN** each occupies a distinct key, and both sit under the same minor-line prefix

#### Scenario: A rebuild does not replace published bytes

- **WHEN** a channel's release is rebuilt and its version carries a build counter distinguishing the rebuild
- **THEN** the rebuild's artifacts occupy new keys and the earlier build's objects are left byte-for-byte intact

#### Scenario: An upload never overwrites

- **WHEN** an upload targets a key that already holds an object
- **THEN** the upload is refused and the run is treated as having failed that target

### Requirement: Each channel elects what it publishes

A channel SHALL publish only what its election admits. A nightly SHALL be elected only when product work landed since the previous nightly; a run finding none SHALL publish nothing and SHALL NOT mint a version. A preview or stable release SHALL publish the tag that triggered the run. A run invoked without a tag SHALL elect nothing, upload nothing, and write no manifest.

#### Scenario: A quiet repository publishes no nightly

- **WHEN** a nightly run finds no product work landed since the previous nightly
- **THEN** no version is minted, no artifact is uploaded, and no manifest is written

#### Scenario: A tagged channel publishes its trigger

- **WHEN** a preview or stable run is triggered by a tag push
- **THEN** that tag's version is the one published, and no other version is elected

#### Scenario: A build-only run publishes nothing

- **WHEN** a run is invoked with no tag
- **THEN** the build proceeds for verification, and nothing is uploaded, advertised, or recorded

### Requirement: A channel advertises a release only once every target has succeeded

Each channel SHALL have a declared set of required targets. A channel's manifest SHALL be written only after every target in that set has uploaded its artifact successfully, and SHALL be the last write of a publishing run. Completeness SHALL be determined by counting successful target uploads against the required set, and SHALL NOT be inferred from the absence of failure — a target that was skipped SHALL block publication exactly as a target that failed.

#### Scenario: A skipped target blocks publication

- **WHEN** a run is invoked for a subset of its channel's required targets, with a tag
- **THEN** no manifest is written and no release record is published, even though no target failed

#### Scenario: A failed target blocks publication

- **WHEN** one target's upload fails and the rest succeed
- **THEN** no manifest is written and no release record is published

#### Scenario: A complete run advertises exactly once

- **WHEN** every required target for a channel has uploaded successfully
- **THEN** the channel's manifest is written, naming that version and its targets, and the release record is published

#### Scenario: The manifest is written last

- **WHEN** a publishing run uploads artifacts and writes a manifest
- **THEN** every artifact the manifest names is already present in the store at the moment the manifest becomes readable

### Requirement: A shortfall withdraws what the run created, and nothing else

A run that fails or falls short of its channel's required targets SHALL discard the release record it drafted. It SHALL delete the release's tag if and only if that run minted the tag itself; a tag that existed before the run — including one whose push triggered the run — SHALL be left in place. Artifacts already uploaded by a shortfall SHALL be left in the store unreferenced rather than deleted, since no manifest names them.

#### Scenario: A self-minted tag is withdrawn

- **WHEN** a run that minted its own tag fails or falls short
- **THEN** the drafted record is discarded and the tag is deleted

#### Scenario: A triggering tag survives

- **WHEN** a run triggered by a tag push fails or falls short
- **THEN** the drafted record is discarded and the tag is left in place

#### Scenario: Uploaded objects are removed, and their removal is not load-bearing

- **WHEN** a shortfall leaves artifacts in the store from targets that did succeed
- **THEN** the run deletes those objects, and if a deletion fails the objects remain unreferenced and unadvertised rather than the withdrawal being treated as incomplete

### Requirement: Every published artifact is retained, and retention changes cost rather than availability

The store SHALL retain every artifact it has ever published, for every channel and every target, including targets that have no download link. Reclaiming storage SHALL be achieved by moving an object to a lower-cost storage class and SHALL NOT be achieved by removing it. An artifact SHALL remain reachable through the same request path whatever class it is held in, and the delivery surface SHALL NOT require a client to initiate a restore before reading one.

#### Scenario: An old release stays available

- **WHEN** an artifact has aged past any retention threshold
- **THEN** it is still retrievable, and a request for it differs from a request for a current artifact only in what it costs to serve

#### Scenario: A quiet period changes nothing

- **WHEN** no release is published for an extended period
- **THEN** every previously published artifact remains retrievable, and none is removed for having aged

#### Scenario: Targets without a download link are still archived

- **WHEN** a target's artifact has no advertised download path
- **THEN** it is uploaded and retained with the rest of its release

#### Scenario: A client never restores an artifact itself

- **WHEN** an artifact in the lower-cost class is requested
- **THEN** it is reached through the same URL as any other artifact, and no separate restore request is asked of the client

### Requirement: The store's write credential is an integrity credential

The credential that writes to the artifact store SHALL be treated as governing integrity, not confidentiality: whoever holds it can place bytes that users install as a signed release. It SHALL be resolved only by the steps that contact the store, under the environment their job declares, and SHALL NOT be exported into the job environment, made available to the build, or reachable by any third-party step or build script in the dependency graph.

#### Scenario: Only the contacting steps resolve it

- **WHEN** a release build job uploads an artifact or writes a manifest
- **THEN** the credential is resolved by that step alone, and no other step in the job can read it

#### Scenario: The build never holds it

- **WHEN** a release build compiles the application
- **THEN** the store credential is absent from the environment that compilation runs in

#### Scenario: Read access needs no credential of its own

- **WHEN** the download surface reads an artifact from the store
- **THEN** it does so under access granted to the deployment rather than a key pair held in configuration

### Requirement: An uploading job holds only a derived, expiring, scoped credential

The long-lived credential that authorises writes SHALL be resolved only to derive a short-lived one, SHALL NOT be held by any client that contacts the store, and SHALL be dropped once the derivation is complete. Every client that uploads a release artifact SHALL be built from a credential that expires, that is confined to the key prefix of the channel the job is publishing, and that permits writing objects without conferring administrative access to the bucket.

The lifetime SHALL be bounded and configurable, so it can follow build durations without a change to the publishing contract. Deriving a credential SHALL NOT depend on reaching a network service, so that no external dependency sits between a completed build and its upload.

#### Scenario: The long-lived credential never reaches a client

- **WHEN** an uploading job prepares to contact the store
- **THEN** the long-lived credential is resolved only to derive a short-lived one, is dropped before any store client exists, and no client is built from it

#### Scenario: A job's credential expires

- **WHEN** a derived credential outlives its configured lifetime
- **THEN** it no longer authorises writes, whether or not the run that minted it has finished

#### Scenario: A job cannot write outside its channel

- **WHEN** a job publishing one channel attempts to write under another channel's prefix
- **THEN** the write is refused by the credential's own scope rather than by a check in the pipeline

#### Scenario: A credential cannot administer the bucket

- **WHEN** a derived credential is used against the store
- **THEN** it can read and write objects within its scope and cannot alter the bucket's own configuration

#### Scenario: Lifetime follows build duration without a contract change

- **WHEN** builds grow long enough that the current lifetime is too short
- **THEN** the lifetime is raised by configuration, and nothing about the key layout, the manifest, or the completeness rule changes

#### Scenario: Minting needs no network service

- **WHEN** a derived credential is produced
- **THEN** it is computed locally from the long-lived credential, so no credential service can fail a release
