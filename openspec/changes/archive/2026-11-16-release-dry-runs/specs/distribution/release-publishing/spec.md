# Spec Delta

## MODIFIED Requirements

### Requirement: Each channel elects what it publishes

A channel SHALL publish only what its election admits. A nightly SHALL be elected only when product work landed since the previous nightly; a run finding none SHALL publish nothing and SHALL NOT mint a version. A preview or stable release SHALL publish the tag that triggered the run, or the existing tag a real re-run names. A dry run SHALL elect nothing to publish: it MAY build for verification, and SHALL upload nothing to the store, publish no record, and write no manifest. A real preview or stable run that names no tag SHALL be refused.

#### Scenario: A quiet repository publishes no nightly

- **WHEN** a nightly run finds no product work landed since the previous nightly
- **THEN** no version is minted, no artifact is uploaded, and no manifest is written

#### Scenario: A tagged channel publishes its trigger

- **WHEN** a preview or stable run is triggered by a tag push
- **THEN** that tag's version is the one published, and no other version is elected

#### Scenario: A build-only run publishes nothing

- **WHEN** a dry run is asked to build
- **THEN** the build proceeds for verification, and nothing is uploaded, advertised, or recorded

#### Scenario: A real run without a tag is refused

- **WHEN** a preview or stable run is dispatched as a real run with no tag
- **THEN** the planner refuses, and nothing is built or published

## ADDED Requirements

### Requirement: A dry run's writes go to the run's own artifacts

A dry run SHALL render what a real run would have written — the release notes, the channel manifest, and the release-index entry — and upload them, with any builds, as artifacts of the run itself. It SHALL write nothing to the artifact store, create or edit no release record, mint no tag, open no pull request, and submit to no app store. Having created nothing, it SHALL withdraw nothing, so a dry run against an existing tag leaves that release's record, objects, and tag intact.

#### Scenario: The rendered writes are readable from the run

- **WHEN** a dry run completes with every required target built
- **THEN** its artifacts hold the notes, the manifest, and the index entry the real run would have written, alongside the builds

#### Scenario: Nothing reaches the store or the forge

- **WHEN** a dry run completes
- **THEN** the artifact store holds no new object, no release record was created or changed, and no tag was created

#### Scenario: A dry run skips the stores

- **WHEN** a dry run reaches the publishing steps
- **THEN** neither app store is contacted, and the point of no return is never marked

#### Scenario: A dry run against a published tag withdraws nothing

- **WHEN** a dry run names a tag whose release is already published and one of its builds fails
- **THEN** the release's record, store objects, and tag are left as they were

#### Scenario: No build, no publish steps

- **WHEN** a dry run is asked not to build
- **THEN** no target records completion, and the publishing job does not run
