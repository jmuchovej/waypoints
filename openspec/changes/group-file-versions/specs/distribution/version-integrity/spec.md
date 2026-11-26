## ADDED Requirements

### Requirement: Group members declare their group's version

Every versioned member of a release group SHALL declare exactly the version its group declares in the group file, prerelease included. A pull request SHALL fail when any member's `[package].version` differs from its group's declared version, and SHALL name the manifest and the group file. The release bot's version writes SHALL set the file and every member of a group in one commit, so a landed tree never holds a manifest that disagrees with the file.

#### Scenario: Lockstep drift

- **WHEN** `crates/cartograph` declares a different version than the group file declares for the app
- **THEN** the check fails and names `crates/cartograph/Cargo.toml` and the group file

#### Scenario: Release bot writes the app group

- **WHEN** the release bot's pull request advances the app to `0.2.0-pre.2`
- **THEN** the same commit sets the group file's app version and `src-tauri` and `crates/cartograph` to `0.2.0-pre.2`, and the check passes

### Requirement: A series' group file names its pins

On a series branch that pins a library group, the group file's `version` for that group SHALL equal the version the pin's tag names, and a pull request leaving them different SHALL fail naming the pin and the group file. A pinned group's in-tree crates are not workspace members and SHALL NOT be compared.

#### Scenario: A series' file names its pins

- **WHEN** a pull request against `v0.2.x`, which pins `bedrock-v0.2.0`, leaves the group file declaring `0.1.0` for `bedrock`
- **THEN** the check fails and names the pin and the group file

#### Scenario: A pinned group has no manifest to compare

- **WHEN** a pull request against `v0.2.x` pins `bedrock-v0.2.0`, the group file declares `0.2.0` for `bedrock`, and the series' in-tree `crates/bedrock` is not a workspace member
- **THEN** the check passes without reading that crate's manifest

## MODIFIED Requirements

### Requirement: Only the release bot writes versions

A pull request SHALL fail when it changes the `[package].version` of a versioned member, or the `version` a group declares in the group file, unless it is the release bot's. A `version` key that appears where none was declared before is not a version write; what it may claim is bounded by the lockstep rule.

#### Scenario: Hand-edited version

- **WHEN** a pull request changes `version` in `crates/domain/Cargo.toml`
- **THEN** the check fails and names the file

#### Scenario: Release bot write

- **WHEN** the release bot's pull request sets every member of `bedrock` to the same new version
- **THEN** the check passes

#### Scenario: Hand-edited group version

- **WHEN** a pull request not authored by the release bot changes `domain`'s `version` in `tooling/release-groups.toml`
- **THEN** the check fails and names the group file

#### Scenario: Release bot writes the file

- **WHEN** the release bot's pull request sets `bedrock`'s `version` in the group file and every `bedrock` member's manifest to the same new version
- **THEN** the check passes

## REMOVED Requirements

### Requirement: Group members declare their anchor's version

**Reason**: Groups no longer name an anchor; the group file declares each group's version, and every member is held to that declaration. The requirement continues under _Group members declare their group's version_, which carries both of its scenarios; _A series' group file names its pins_ adds the pin agreement a series needs.

**Migration**: Remove `anchor` from `tooling/release-groups.toml` and declare `version` per group; run `cargo xtask generate versions` so every member manifest takes its group's declared version.
