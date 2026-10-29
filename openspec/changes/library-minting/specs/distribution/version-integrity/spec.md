## ADDED Requirements

### Requirement: A series branch takes library changes only by pin

A pull request against a series branch SHALL fail when it changes any file under a library group's own paths. A series builds its libraries from the tags it pins, so such a change could not reach its artifacts; the check SHALL name the path and the pin that governs it. A release bot pull request that moves a pin SHALL pass. Every crate that depends on a group SHALL pin the same tag, since one binary cannot carry two copies of a group.

#### Scenario: Library code cherry-picked onto a series

- **WHEN** a pull request against `v0.2.x` changes `crates/bedrock/src/query.rs`
- **THEN** the check fails, names the file, and names the `bedrock` pin as what the series builds from

#### Scenario: Pin moved by the release bot

- **WHEN** the release bot's pull request against `v0.2.x` moves every `bedrock` pin from `bedrock-v0.2.0` to `bedrock-v0.2.1`
- **THEN** the check passes

#### Scenario: Pins diverge

- **WHEN** a pull request leaves `src-tauri` pinned to `bedrock-v0.2.0` and `crates/store` pinned to `bedrock-v0.1.0`
- **THEN** the check fails and names both manifests, since one binary cannot carry two copies of a group

#### Scenario: A series that still builds in-tree

- **WHEN** a pull request against a series whose manifests pin no library group — one cut before any library tag existed — changes `crates/bedrock/src/query.rs`
- **THEN** the check passes: the series builds that file from its own tree, and there is no pin for the change to bypass

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
