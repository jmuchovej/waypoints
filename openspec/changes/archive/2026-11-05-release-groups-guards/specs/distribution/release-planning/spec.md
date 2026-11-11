## ADDED Requirements

### Requirement: Every workspace crate is named in the group file

Every workspace member outside `tooling/` SHALL be named in `tooling/release-groups.toml` as a versioned member of exactly one group or an unversioned internal of exactly one group. Members under `tooling/` build the repository, not the product, and SHALL NOT be named. Each group SHALL name an anchor whose `[package].version` is the group's version. A planner SHALL refuse to run while any member outside `tooling/` is unnamed, any member is named twice, or any member under `tooling/` is named.

#### Scenario: Unnamed crate

- **WHEN** `crates/importer` is a workspace member and the file does not name it
- **THEN** the planner refuses to run and names the crate

#### Scenario: Crate in two groups

- **WHEN** one crate is named by two groups
- **THEN** the planner refuses to run and names both groups

#### Scenario: Tooling crates are out of scope

- **WHEN** `tooling/xtask` and `tooling/gh-workflow` are workspace members and the file names neither
- **THEN** the planner runs; and if the file did name one, it would refuse and name the crate

#### Scenario: New crate joins the workspace

- **WHEN** a crate such as `crates/transload` becomes a workspace member
- **THEN** every planner run refuses until the file names it
