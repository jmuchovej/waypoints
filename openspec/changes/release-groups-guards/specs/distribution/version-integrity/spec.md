## Purpose

Defines the pull-request checks that make every version claim verifiable before it reaches `main` or a series branch — who may write a version, how a landing message must be shaped, and what evidence a declared severity must agree with — so that the planners' output is the record of what shipped rather than a reading of whatever was typed.

## ADDED Requirements

### Requirement: The group file's schema is current and enforced

`tooling/schemas/release-groups.schema.json` SHALL equal the schema generated from the planner's config types, and `tooling/release-groups.toml` SHALL bind it with a `#:schema` directive and be linted against it.

#### Scenario: Stale schema

- **WHEN** a field is added to the planner's config types without regenerating the schema
- **THEN** the check fails

#### Scenario: Misspelled key

- **WHEN** the group file contains `attribution` instead of a known key
- **THEN** the TOML lint fails

### Requirement: Only the release bot writes versions

A pull request SHALL fail when it changes the `[package].version` of a versioned member, unless it is the release bot's.

#### Scenario: Hand-edited version

- **WHEN** a pull request changes `version` in `crates/domain/Cargo.toml`
- **THEN** the check fails and names the file

#### Scenario: Release bot write

- **WHEN** the release bot's pull request sets every member of `bedrock` to the same new version
- **THEN** the check passes

### Requirement: Landing commits are conventional, and summaries are complete

Every pull request SHALL land on `main` or a series branch as a merge commit whose message is the pull request's title and description, and that message SHALL be a conventional commit. A pull request changing more than one group's own paths SHALL be a summary with exactly one header per touched group. A `chore(release):` subject SHALL be reserved to the release bot. Branch commits under the merge SHALL NOT be checked.

#### Scenario: Free-form message

- **WHEN** a pull request's title is `Fix sync bug`
- **THEN** the check fails

#### Scenario: Summary missing a group

- **WHEN** a summary changes `crates/bedrock/` and `crates/domain/` but carries only a `bedrock` header
- **THEN** the check fails and names `domain`

#### Scenario: Summary naming an untouched group

- **WHEN** a summary changes only `crates/bedrock/` but carries headers for `bedrock` and `outpost`
- **THEN** the check fails and names `outpost`

#### Scenario: Reserved bookkeeping subject

- **WHEN** a pull request not authored by the release bot is titled `chore(release): …`
- **THEN** the check fails

### Requirement: Covenant fixtures are append-only unless declared

A pull request that modifies, deletes, or renames a file under `covenant/fixtures/` SHALL fail unless it carries `Covenant: break`. Adding fixtures SHALL pass. Fixtures SHALL be excluded from the formatting pipeline, so a formatter cannot rewrite one.

#### Scenario: New fixture

- **WHEN** a pull request only adds files under `covenant/fixtures/domain/`
- **THEN** the check passes

#### Scenario: Modified fixture without the trailer

- **WHEN** a pull request modifies an existing fixture without `Covenant: break`
- **THEN** the check fails and names the fixture

#### Scenario: Modified fixture with the trailer

- **WHEN** a pull request modifies an existing fixture and carries `Covenant: break`
- **THEN** the check passes

#### Scenario: Formatter leaves fixtures alone

- **WHEN** the formatting pipeline runs over the tree
- **THEN** no file under `covenant/fixtures/` changes
