## ADDED Requirements

### Requirement: A channel's release is built and tested under its own flavor

Every release on every channel SHALL be a fresh build under that channel's flavor; a promotion SHALL rebuild under the stable flavor rather than relabel the preview build. The release test suite SHALL run under the same flavor the build enables, so the compiled combination that ships is the one that was tested. A shipped flavor MAY gate behaviour at compile time; what distinguishes channels is for each channel's own build to carry.

#### Scenario: Promotion rebuilds

- **WHEN** `v0.2.x` is promoted from `0.2.0-pre.7` to `0.2.0`
- **THEN** the stable artifacts come from a build under the stable flavor, after the release suite has run under that flavor, and no preview artifact is relabelled

#### Scenario: The suite runs under the shipped flavor

- **WHEN** the nightly release suite gates the nightly builds
- **THEN** it ran with the nightly flavor enabled on every crate that declares one, and a nightly-only code path is exercised by it

#### Scenario: A pull request is tested under its branch's channel

- **WHEN** a pull request targets `main`, whose manifest declares a nightly version
- **THEN** its test suite runs with the nightly flavor enabled; a pull request against a series branch runs with that branch's channel flavor

#### Scenario: Gates form a staircase

- **WHEN** an item is gated to the preview flavor and another to the nightly flavor
- **THEN** a nightly build compiles both, a preview build compiles the preview-gated item alone, and a stable build compiles neither

### Requirement: A build enables exactly one shipped flavor

A build SHALL enable exactly one shipped flavor, and a crate that gates on flavors SHALL refuse to compile with two shipped flavors enabled, since Cargo features are additive and flavors are not. The development flavor is not a channel: it MAY be enabled beside a shipped flavor by test and development builds, SHALL only add to a channel's behaviour and never override a channel gate, and SHALL NOT reach a released artifact.

#### Scenario: Two flavors refused

- **WHEN** feature unification would enable both `preview` and `stable` on one crate
- **THEN** the crate fails to compile, naming the conflict

#### Scenario: The wrong flavor for the tree

- **WHEN** `main` declares `0.3.0-nightly` and a build enables `stable` alone
- **THEN** the build is refused, naming `stable` and the nightly channel the manifest declares

#### Scenario: Development beside a shipped flavor

- **WHEN** `main` declares `0.3.0-nightly` and a workstation build enables `development` and `stable`
- **THEN** the build compiles stable's baseline with the development additions, and nothing treats it as a release

#### Scenario: A release build without a shipped flavor

- **WHEN** a release build enables no shipped flavor
- **THEN** the build is refused, naming the flavors it accepts

#### Scenario: A test build without a shipped flavor

- **WHEN** `cargo test` or a dev server compiles the app with no shipped flavor enabled
- **THEN** the compile succeeds on the ungated baseline, since it produces no release build
