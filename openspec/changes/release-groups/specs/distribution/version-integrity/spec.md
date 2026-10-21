## Purpose

Defines the pull-request checks that make every version claim verifiable before it reaches `main` or a series branch — who may write a version, how a landing message must be shaped, and what evidence a declared severity must agree with — so that the planners' output is the record of what shipped rather than a reading of whatever was typed.

## ADDED Requirements

### Requirement: Only the release bot writes versions

A pull request SHALL fail when it changes the `[package].version` of a versioned member, unless it is the release bot's. Members of a group SHALL declare their anchor's version.

#### Scenario: Hand-edited version

- **WHEN** a pull request changes `version` in `crates/domain/Cargo.toml`
- **THEN** the check fails and names the file

#### Scenario: Lockstep drift

- **WHEN** `crates/cartograph` declares a different version than `src-tauri`
- **THEN** the check fails and names both manifests

#### Scenario: Release bot write

- **WHEN** the release bot's pull request sets every member of `bedrock` to the same new version
- **THEN** the check passes

#### Scenario: Pins diverge

- **WHEN** a pull request leaves `src-tauri` pinned to `bedrock-v0.2.0` and `crates/store` pinned to `bedrock-v0.1.0`
- **THEN** the check fails and names both manifests, since one binary cannot carry two copies of a group

### Requirement: Landing commits are conventional, and summaries are complete

Every pull request SHALL land on `main` or a series branch as a merge commit whose message is the pull request's title and description, and that message SHALL be a conventional commit. A pull request changing more than one group's own paths SHALL be a summary with exactly one header per touched group. A `chore(release):` subject SHALL be reserved to the release bot. Branch commits under the merge SHALL NOT be checked. The release bot's own pull requests SHALL be gated by the same required checks as any other.

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

#### Scenario: Release-style pull request

- **WHEN** the release bot opens a pull request writing versions and one required check fails
- **THEN** the pull request does not merge

### Requirement: An undeclared API break fails the pull request

For each library group a pull request touches, a break detected by the API-compatibility check against that group's last tag SHALL fail the check unless the group's header declares a break. A declared break with no detected break SHALL pass.

#### Scenario: Break in an undeclared group

- **WHEN** a summary declares `feat(bedrock)!: …` and `refactor(outpost): …`, and introduces an API break in `outpost`
- **THEN** the check fails and names `outpost`

#### Scenario: Over-declared break

- **WHEN** `fix(bedrock)!: …` has no detected API break
- **THEN** the check passes

#### Scenario: App API break is information only

- **WHEN** a pull request introduces an API break in `crates/cartograph` without a `Covenant: break` trailer
- **THEN** the check reports it and passes, since the app's contract is the covenant, not a Rust API

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

### Requirement: The group file's schema is current and enforced

`tooling/schemas/release-groups.schema.json` SHALL equal the schema generated from the planner's config types, and `tooling/release-groups.toml` SHALL bind it with a `#:schema` directive and be linted against it.

#### Scenario: Stale schema

- **WHEN** a field is added to the planner's config types without regenerating the schema
- **THEN** the check fails

#### Scenario: Misspelled key

- **WHEN** the group file contains `attribution` instead of a known key
- **THEN** the TOML lint fails
