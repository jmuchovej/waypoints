# Spec Delta

## ADDED Requirements

### Requirement: Group members declare their anchor's version

Every versioned member of a release group SHALL declare exactly the version its group's anchor declares, prerelease included. A pull request SHALL fail when any member's `[package].version` differs from its anchor's, and SHALL name both manifests. The release bot's version writes SHALL set every member of a group in one commit, so a landed tree never holds a group whose members disagree.

#### Scenario: Lockstep drift

- **WHEN** `crates/cartograph` declares a different version than `src-tauri`
- **THEN** the check fails and names both manifests

#### Scenario: Release bot writes the app group

- **WHEN** the release bot's pull request advances `src-tauri` to `0.2.0-pre.2`
- **THEN** the same commit sets `crates/cartograph` to `0.2.0-pre.2`, and the check passes

### Requirement: The release bot's pull requests are gated like any other

A pull request the release bot opens SHALL be gated by the same required checks as any other pull request, SHALL merge only when every required check has passed, and SHALL merge without any person's intervention once they have. No actor SHALL bypass a required check on `main` or a series branch.

#### Scenario: Release-style pull request

- **WHEN** the release bot opens a pull request writing versions and one required check fails
- **THEN** the pull request does not merge

#### Scenario: Release-style pull request merges on green

- **WHEN** every required check on the release bot's pull request has passed
- **THEN** it lands as a merge commit without anyone approving or merging it by hand
