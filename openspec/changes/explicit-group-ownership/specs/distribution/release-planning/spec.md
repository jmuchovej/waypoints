## ADDED Requirements

### Requirement: The app's group is named `waypoints`

The group file SHALL carry `[group.waypoints]`, and every reader that tells the app's group from a library group SHALL do so by that name alone. A planner SHALL refuse to run on a file without that group, naming it. Every rule a group follows is decided by whether it is the app, so the file SHALL carry no key restating one, and a key nothing reads SHALL NOT parse.

#### Scenario: A file without the app's group

- **WHEN** the group file names `bedrock`, `domain`, `outpost`, and `transload` and no `waypoints`
- **THEN** the planner refuses to run and names `waypoints` as the group it requires

#### Scenario: The app is told from a library by name

- **WHEN** the API check measures the group named `waypoints`, or a build embeds group versions, or the overlay reads the tree's channel
- **THEN** each finds the app's group as `waypoints`: the check leaves the app's own crate out of its measurement and reports its breaks as information, the build embeds the four library groups and not the app, and the overlay reads the channel from `waypoints`' declared version

#### Scenario: A retired key does not parse

- **WHEN** the group file carries `owns`, `counts`, `breaking`, `minted`, `floors`, `features-always-increment-minor`, `channels`, `flavors`, or an `[enforce]` table
- **THEN** the planner refuses to run and names the key, as it names a misspelled one

### Requirement: A group owns only the paths the group file lists

A group SHALL own its members', internals', and fixtures directories and the directories its `paths` key lists, matched as directory prefixes with the longest winning. A commit SHALL count for exactly the groups whose owned paths it changes; a path no group lists, a path under `tooling/`, and a bookkeeping path SHALL count for none. A planner SHALL refuse a listed path that does not exist, sits under `tooling/`, or overlaps another owned path, naming the path and the group.

#### Scenario: Library-only change

- **WHEN** a commit changes only files under `crates/bedrock-core/`
- **THEN** it counts toward `bedrock` and toward no other group

#### Scenario: Frontend change

- **WHEN** a commit changes only files under `src/`
- **THEN** it counts toward `waypoints` and toward no other group

#### Scenario: Unlisted path

- **WHEN** a commit changes only files under `website/`, `docs/`, `nix/`, or another directory no group lists
- **THEN** it counts toward no group, and a day whose commits all change such paths publishes no nightly

#### Scenario: Summary across two libraries

- **WHEN** a summary changes files under `crates/bedrock/` and `crates/outpost/` and no path the app owns
- **THEN** it counts toward `bedrock` and `outpost`, and not toward `waypoints`

#### Scenario: Listed path that does not exist

- **WHEN** a group's `paths` lists a directory the tree does not hold
- **THEN** the planner refuses to run and names the path and the group

#### Scenario: Listed path under tooling

- **WHEN** a group's `paths` lists a directory under `tooling/`
- **THEN** the planner refuses to run and names the path and the group

#### Scenario: Overlapping listed path

- **WHEN** a group's `paths` lists a directory inside another group's member, or one that holds another group's owned path
- **THEN** the planner refuses to run and names both paths

### Requirement: A cut with nothing counted for the app refuses

A cut SHALL refuse when no commit since the last stable release counts for the app's group, before reading `--major`, and SHALL name the last stable tag or say that none exists, name each library group whose commits would mint, and point at the group-scoped cut as the way to mint one alone. A cut SHALL NOT compute an app target from no counted commit. A group-scoped cut SHALL be unaffected.

#### Scenario: Library-only cycle

- **WHEN** the last stable is `v0.1.0`, `bedrock` and `domain` have counted commits since their tags, and no commit since `v0.1.0` counts for `waypoints`
- **THEN** the cut refuses, names `v0.1.0`, names `bedrock` and `domain`, and points at `release-plan cut --group <name>`

#### Scenario: Before the first stable

- **WHEN** no stable tag exists and no commit in the history counts for `waypoints`
- **THEN** the cut refuses, says there is no stable tag, and names the library groups that would mint

#### Scenario: A group-scoped cut still mints

- **WHEN** no commit counts for `waypoints` and the cut is scoped to `bedrock`, which has counted commits since its tag
- **THEN** `bedrock` is minted alone, as a group-scoped cut always is

## REMOVED Requirements

### Requirement: Groups own disjoint paths, and the app counts what it ships

**Reason**: The app no longer counts every group's paths. A group owns the paths the group file lists for it and a commit counts for exactly the groups whose owned paths it changes; the app's version moves on the app's own paths, the app adopting a library change is a commit on those paths, and a series takes a newer library by a pin move. The ownership half of this requirement continues under _A group owns only the paths the group file lists_, which carries its _Library-only change_ scenario with the app removed from the answer; its _Tooling-only change_, _Treewide formatting pass_, and _Release bot bookkeeping_ scenarios remain true under the bookkeeping rule that requirement restates. This requirement is in `library-minting`'s open delta, so this removal applies once that change has archived.

**Migration**: None for landed history; the walk reads the same commits with narrower attribution. A reader who wants a library change to move the app's version lands the commit that adopts it on an app path, or cuts the app deliberately.
