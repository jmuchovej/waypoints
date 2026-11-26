## MODIFIED Requirements

### Requirement: Landing commits are conventional, and summaries are complete

Every pull request SHALL land on `main` or a series branch as a merge commit whose message is the pull request's title and description, and that message SHALL be a conventional commit. A pull request changing more than one group's own paths — the members, internals, fixtures directory, and `paths` the group file lists for each group — SHALL be a summary with exactly one header per touched group; a path no group lists touches no group. A `chore(release):` subject SHALL be reserved to the release bot. Branch commits under the merge SHALL NOT be checked.

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

#### Scenario: Frontend and library in one pull request

- **WHEN** a pull request titled `feat(app): take the new filter` changes `src/app/a.vue` and `crates/bedrock/src/lib.rs`
- **THEN** the check fails, names `waypoints` and `bedrock`, and asks for a `summary: …` with one header per group

#### Scenario: An unlisted path beside a library

- **WHEN** a pull request titled `fix(bedrock): a bug` changes `website/a.md` and `crates/bedrock/src/lib.rs`
- **THEN** the check passes: `website/` is listed by no group, so the pull request touches `bedrock` alone
