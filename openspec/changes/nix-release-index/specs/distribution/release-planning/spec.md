## Purpose

Defines the invariants the release and nightly planners must hold when deciding whether and what to publish, so that the bookkeeping a release produces cannot itself be mistaken for the product work that justifies the next release.

## ADDED Requirements

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
