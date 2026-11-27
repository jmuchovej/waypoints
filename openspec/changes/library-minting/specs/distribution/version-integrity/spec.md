## ADDED Requirements

### Requirement: An undeclared API break fails the pull request

For each library group a pull request touches, a break detected by the API-compatibility check against that group's last tag SHALL fail the check unless the group's header declares a break. A declared break with no detected break SHALL pass. The check SHALL measure the surface compiled under the stable flavor. A group with no tag SHALL be skipped, and the check SHALL say so naming the group.

#### Scenario: Break in an undeclared group

- **WHEN** a summary declares `feat(bedrock)!: …` and `refactor(outpost): …`, and introduces an API break in `outpost`
- **THEN** the check fails and names `outpost`

#### Scenario: Over-declared break

- **WHEN** `fix(bedrock)!: …` has no detected API break
- **THEN** the check passes

#### Scenario: App API break is information only

- **WHEN** a pull request introduces an API break in `crates/cartograph` without a `Covenant: break` trailer
- **THEN** the check reports it and passes, since the app's contract is the covenant, not a Rust API

#### Scenario: No tag to measure against

- **WHEN** a pull request touches `crates/transload` and no `transload-v*` tag exists
- **THEN** the check skips `transload`, reports that it has no baseline until the first cut mints one, and passes on that ground
