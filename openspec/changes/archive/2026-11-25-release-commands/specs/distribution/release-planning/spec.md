## ADDED Requirements

### Requirement: A bump infers the transition from the series

A bump SHALL be asked for a channel or a library group, never a transition. For a channel it SHALL read the open series from the remote and infer: a preview bump advances a series in preview, cuts one when the newest is stable, and refuses a cut while a preview is open; a stable bump promotes a preview, patches a stable series with new cherry-picks, and refuses otherwise. A declared version no tag names is a withdrawal in flight, and SHALL be refused. Only a cut SHALL size from history.

#### Scenario: A preview bump on an open preview

- **WHEN** the newest series declares `0.2.0-pre.3`, tagged, counted work landed since that tag, and a preview bump runs
- **THEN** it advances the series to `0.2.0-pre.4`

#### Scenario: A preview bump after a stable

- **WHEN** the newest series is stable and a preview bump runs
- **THEN** it cuts the next series from `main`, sized from `main`'s history since the last stable

#### Scenario: A stable bump on a preview

- **WHEN** the newest series declares a preview and a stable bump runs
- **THEN** it promotes the series in place

#### Scenario: A stable bump with nothing to patch

- **WHEN** the newest series is stable, tagged, and carries no counted cherry-pick since its tag
- **THEN** the bump refuses, naming the series and its tag

#### Scenario: A withdrawal in flight

- **WHEN** a series' group file declares a version no tag names, because a withdrawal's revert has not landed
- **THEN** the bump refuses, naming the version and the open revert, and moves nothing

#### Scenario: A cut's write without its series

- **WHEN** `main` declares a nightly target two minors above the newest series, because a withdrawn cut's revert has not landed
- **THEN** every bump, nightly included, refuses until the revert lands

#### Scenario: A library bump

- **WHEN** a bump is asked for a library group with a floor
- **THEN** it mints the group's tag on `main`, raises the bump to the floor, and opens no series

### Requirement: Bookkeeping alone moves no tag

A bump SHALL move or create a channel's tag only when counted product work landed since the channel's previous tag: on `main` since the last nightly, on a series since its last tag. A bump that finds only bookkeeping, or nothing, SHALL refuse or skip, naming the tag it would have moved, and SHALL create no ref. A promotion is exempt: it declares a candidate final and moves no work. The refusal is the bump's alone; nothing downstream is told.

#### Scenario: A preview bump with nothing new

- **WHEN** the newest series declares `0.2.0-pre.3`, tagged, and only bookkeeping commits landed since that tag
- **THEN** the bump refuses naming `v0.2.0-pre.3`, and no ref is created

#### Scenario: A promotion needs no new work

- **WHEN** the newest series declares a preview, tagged, with nothing landed since, and a stable bump runs
- **THEN** it promotes the series
