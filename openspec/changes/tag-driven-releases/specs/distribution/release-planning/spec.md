## ADDED Requirements

### Requirement: One workflow bumps every channel and every group

One workflow SHALL run every bump: its schedule SHALL bump nightly, and its dispatch SHALL take a target that is a channel or a library group, the major flag, the floor, and a rehearsal flag that defaults on. A nightly dispatch SHALL always rehearse. The workflow SHALL land each write through a pull request, wait for the merge in the run, and tag the merge commit. It SHALL share one queued concurrency group with the release workflow, so one release cycle runs at a time.

#### Scenario: A nightly waits out a release

- **WHEN** the schedule fires while a release run is in progress
- **THEN** the nightly bump queues behind it and runs when it finishes

#### Scenario: The schedule mints a nightly

- **WHEN** the bump workflow runs on its schedule
- **THEN** it bumps nightly for real, minting the day's tag when work landed

#### Scenario: A dispatch names its target

- **WHEN** the bump workflow is dispatched with `stable` as its target and the rehearsal flag off
- **THEN** it runs the stable bump for real and lands the write

#### Scenario: A group is a target

- **WHEN** the bump workflow is dispatched with a library group as its target
- **THEN** it runs the group bump with the chosen floor

## MODIFIED Requirements

### Requirement: A dry run decides as a real run and mints nothing

A dry run SHALL reach the decision a real run would reach on the same tree, tags, and date, SHALL create no ref, and SHALL report the tag and version it would have minted. Asked to build, it SHALL build the day's version whatever it decided. A run that does not build SHALL only be a dry run, since a tag with no artifacts behind it must not be minted. A dry run SHALL happen only by dispatch, never on a schedule.

#### Scenario: A dry run reports the would-be tag

- **WHEN** the nightly planner runs dry on a day with product work since the latest nightly
- **THEN** it reports `v{target}-nightly.{date}` as the tag it would mint, marks it as not minted, and no ref is created

#### Scenario: A dry run builds when asked, whatever the decision

- **WHEN** the release pipeline is dispatched dry from a nightly tree with building asked for, and no product work landed since the latest nightly
- **THEN** the day's version is built all the same, and nothing is published

#### Scenario: No-build without dry-run is refused

- **WHEN** a planner is asked not to build without being asked for a dry run
- **THEN** it refuses before reading the tree, and nothing is decided, minted, or built

#### Scenario: A nightly dispatch is always dry

- **WHEN** the bump workflow is dispatched by hand with nightly as its target
- **THEN** the planner runs dry; only the schedule mints a nightly

#### Scenario: A dispatch defaults to dry

- **WHEN** the bump workflow or the release workflow is dispatched without choosing otherwise
- **THEN** the run is a dry run, and a real run has to be asked for explicitly
