# Spec Delta

## MODIFIED Requirements

### Requirement: The app manifest is the sole source of a build's version

Every build's version SHALL be read from the app manifest (`src-tauri/Cargo.toml`) of the commit being built. No other file SHALL supply or override it — in particular, a flavor's Tauri config SHALL NOT declare a `version`, since a Tauri config `version` takes precedence over the manifest and would pin every branch's bundle to one value.

This governs every distribution channel, not any single packaging mechanism: the bundler pipeline's platform artifacts, the store build numbers, the identity a build reports about itself, and the Nix packages alike.

A publish MAY append a prerelease suffix that exists only at publish time — a nightly's date-stamped identifier is assigned by the planner and appears in no manifest. Such a suffix SHALL extend the manifest's version rather than replace it, so the base version of a published artifact always equals the manifest version of the commit it was built from.

#### Scenario: Version comes from the built commit's manifest

- **WHEN** a build is produced from a commit whose app manifest declares version `X`
- **THEN** the artifact reports version `X`, optionally extended by a publish-time suffix, and no flavor config overrides it

#### Scenario: Publish-time suffix extends rather than replaces

- **WHEN** a nightly is published from a commit whose manifest declares `X`
- **THEN** its version is `X` plus the planner's date-stamped prerelease identifier, and stripping that suffix yields exactly `X`

## ADDED Requirements

### Requirement: A nightly is named by its UTC date

A nightly's version SHALL be `{target}-nightly.{YYYYMMDD}`, where the identifier is the planner's UTC date, and SHALL carry no build metadata. Nightlies of one target therefore order by date under semantic versioning alone, and at most one nightly SHALL exist per UTC day. The store build numbers SHALL be derived from that identifier. A nightly SHALL be read only in that shape: a nightly tag that does not fit it SHALL fail the planner, naming the tag, rather than be read or ignored.

#### Scenario: A date-named version

- **WHEN** the planner publishes a nightly of target `0.1.0` on 2026-11-09 UTC
- **THEN** the version is `0.1.0-nightly.20261109`, the tag is `v0.1.0-nightly.20261109`, and no build metadata is attached

#### Scenario: Dates order without a tie

- **WHEN** `0.1.0-nightly.20261109` and `0.1.0-nightly.20261108` are compared
- **THEN** the later date is the greater version, with no build metadata to break a tie

#### Scenario: An earlier-grammar tag is refused

- **WHEN** the published tags include `v0.1.0-nightly.2+2026.11.08`
- **THEN** the planner fails naming that tag, and no decision is reported

#### Scenario: A second run on a taken day

- **WHEN** the planner runs on a day whose nightly tag already exists for the target, and product work landed since it
- **THEN** it mints no tag and publishes nothing, reporting that the day's nightly exists and the work ships with the next scheduled nightly

#### Scenario: Build numbers come from the identifier

- **WHEN** a nightly's version is `0.2.0-nightly.20260829`
- **THEN** Android's `versionCode` is `20260829` and iOS's `CFBundleVersion` is `2026.08.29`

### Requirement: A dry run decides as a real run and mints nothing

A dry run SHALL reach the decision a real run would reach on the same tree, tags, and date, SHALL create no ref, and SHALL report the tag and version it would have minted. Asked to build, it SHALL build the day's version whatever it decided. A run that does not build SHALL only be a dry run, since a tag with no artifacts behind it must not be minted. A dry run SHALL happen only by dispatch, never on a schedule.

#### Scenario: A dry run reports the would-be tag

- **WHEN** the nightly planner runs dry on a day with product work since the latest nightly
- **THEN** it reports `v{target}-nightly.{date}` as the tag it would mint, marks it as not minted, and no ref is created

#### Scenario: A dry run builds when asked, whatever the decision

- **WHEN** the nightly planner runs dry with building asked for, and no product work landed since the latest nightly
- **THEN** it reports that a scheduled run would skip, and the day's version is built all the same

#### Scenario: No-build without dry-run is refused

- **WHEN** a planner is asked not to build without being asked for a dry run
- **THEN** it refuses before reading the tree, and nothing is decided, minted, or built

#### Scenario: A nightly dispatch is always dry

- **WHEN** the nightly workflow is dispatched by hand
- **THEN** the planner runs dry; only the schedule publishes a nightly

#### Scenario: A dispatch defaults to dry

- **WHEN** the preview, stable, release-plan, or release-index workflow is dispatched without choosing otherwise
- **THEN** the run is a dry run, and a real run has to be asked for explicitly

### Requirement: A dry-run transition reports its refs and renders its writes

A release-cycle transition run dry SHALL name the series branch, the tag, and each pull request it would open (head, title, body), SHALL render the version writes it would commit and the release notes for the would-be tag as artifacts of the run, and SHALL create no branch, pull request, or tag, leaving the working tree unchanged.

#### Scenario: A dry cut

- **WHEN** `cut` runs dry from `main` at `0.1.0-nightly` with a feature landed since the last stable
- **THEN** it names `v0.2.x`, `v0.2.0-pre.1`, and the series and `main` pull requests, renders both version-write diffs and the notes, and the remote gains no branch, pull request, or tag

#### Scenario: The tree is clean afterwards

- **WHEN** a dry transition finishes
- **THEN** no manifest or lockfile in the working tree differs from the checked-out commit

### Requirement: Planners discover tags from the remote

A planner SHALL read the published tags from the repository's remote rather than from the local tag list, so a run from a workstation and a run in the pipeline see one set of tags and decide alike. A planner that cannot reach the remote SHALL fail rather than decide on local tags.

#### Scenario: Stale local tags do not vote

- **WHEN** a workstation holds nightly tags the remote has deleted and the planner runs dry
- **THEN** it decides from the remote's tags and names the same latest nightly the pipeline would

#### Scenario: An unreachable remote

- **WHEN** the remote cannot be reached
- **THEN** the planner fails naming the remote, and no decision is reported
