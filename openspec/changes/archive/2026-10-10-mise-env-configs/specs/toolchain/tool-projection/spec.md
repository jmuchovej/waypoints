## Purpose

Defines how the repository declares the tools that build, test, and format it, and how one declaration becomes the mise configuration consumed by anyone working outside the devenv shell — principally CI. Establishes that a tool's version has exactly one source, that a consumer can ask for a named subset rather than the whole toolchain, and that a subset is never internally incomplete.

## ADDED Requirements

### Requirement: A tool's version has a single source

Every tool the repository provides SHALL be declared once, carrying the nix package that provides it in the devenv shell and the backend-qualified mise spec that provides it elsewhere. The projected mise configuration SHALL take each tool's version from its nix package. No committed file SHALL restate a tool's version independently, and no generated file SHALL be hand-edited to change one.

A tool MAY declare that it belongs to no configuration at all, in which case it exists in the shell only and SHALL be absent from every projected configuration. That declaration SHALL be the absence of environment membership rather than a separate flag, so a tool is projected exactly when it says where it belongs and the two cannot disagree. A tool declared for projection without both halves SHALL be a build-time error, not a silently omitted entry.

A shell-only tool MAY still be read by other generation — for a version it is the single source of — so its declaration is not evidence that it is unused.

#### Scenario: Version follows the nix package

- **WHEN** a tool's nix package is updated to a new version and the projection is regenerated
- **THEN** every projected configuration naming that tool reports the new version, with no other file edited

#### Scenario: Incomplete declaration fails closed

- **WHEN** a tool is declared for projection with a nix package but no mise spec
- **THEN** evaluation fails with a message naming the tool, rather than producing a configuration that omits it

### Requirement: Environments name units of work and compose additively

The projection SHALL emit a base configuration plus one configuration per declared environment. The base SHALL always be loaded; selecting environments SHALL add to it and SHALL NOT mask or remove anything the base declares.

A consumer SHALL be able to select more than one environment at once, receiving the union of the base and every selected environment. An environment name that is not declared SHALL be a build-time error where it is written, because selecting an unmaterialised environment degrades silently to base-only rather than failing.

Every declared environment SHALL emit a configuration file, including one whose members are all in the base. A consumer's selection is then independent of what a lane currently holds: the same selection stays correct as tools move between the base and that lane, and a lane-specific tool added later lands in a file consumers already select.

#### Scenario: Base is always present

- **WHEN** any environment, or none, is selected
- **THEN** every tool declared in the base resolves

#### Scenario: Environments compose

- **WHEN** two environments are selected together
- **THEN** the resolved tool set is the union of the base and both environments

#### Scenario: Undeclared environment name is rejected at source

- **WHEN** a tool or a consumer names an environment that has not been declared
- **THEN** generation fails naming the unknown environment, rather than emitting a configuration that loads nothing for it

### Requirement: An environment is asserted closed over tool dependencies

A tool MAY declare that it depends on other projected tools, naming each by the spec that tool itself declares, so a rename cannot leave a dangling reference. No environment SHALL contain a tool without the runtime that tool executes on.

Generation SHALL verify this and fail where it does not hold; it SHALL NOT satisfy it by adding the missing dependency. An environment's emitted configuration is then exactly what its members declare, so reading the declarations tells you what the file contains. The failure SHALL name the lane, the dependent tool, and the declaration to edit.

Verification is required because the underlying tool manager does not enforce it: a configuration naming an absent dependency resolves the dependent tool and reports success, so an unverified closure produces a configuration that passes every check and fails only when the tool is invoked.

Each lane SHALL be verified against its own members plus the base, since an environment's configuration is loaded on top of the base rather than instead of it.

A dependency naming no projected tool's spec SHALL be a build-time error.

#### Scenario: Missing runtime is rejected at source

- **WHEN** an environment contains a tool whose declared dependency is a member of neither that environment nor the base
- **THEN** generation fails, naming the environment, the dependent tool, and the declaration that must gain the environment

#### Scenario: A dependency satisfied by the base is accepted

- **WHEN** an environment contains a runtime-dependent tool whose runtime is declared in the base
- **THEN** generation succeeds, and the environment's configuration does not restate the runtime

#### Scenario: Closure is load-bearing, not decorative

- **WHEN** an environment containing a runtime-dependent tool is selected in a clean checkout and that tool is invoked
- **THEN** the tool runs, rather than failing with its runtime not found

#### Scenario: Dangling dependency is rejected at source

- **WHEN** a tool declares a dependency on a spec no projected tool carries
- **THEN** generation fails naming the unresolved dependency

### Requirement: Consumers select environments; they never exclude tools

A consumer of the projection SHALL obtain its toolchain by naming the environments whose work it performs. No consumer SHALL obtain its toolchain by enumerating tools to exclude from a larger set.

An exclusion list that names a tool absent from the configuration has no effect and reports no error, so it decays silently as tool specs change; a selection that names a missing environment is caught where it is written, by the preceding requirement.

#### Scenario: A job names what it does

- **WHEN** a generated CI job's toolchain is inspected
- **THEN** it names one or more environments, and names no tool to disable

#### Scenario: Omitted tooling fails visibly

- **WHEN** a consumer selects environments that do not contain a tool it invokes
- **THEN** the invocation fails with that tool not found, rather than proceeding with a substitute

### Requirement: Generated configuration is committed and verified in lockstep

Every projected configuration SHALL be committed, and a check SHALL assert that the committed files match what the current declarations produce. The set of files that check covers SHALL be derived from the generator rather than enumerated by hand, so adding an environment extends the check without a separate edit.

#### Scenario: Hand-edited projection is caught

- **WHEN** a projected configuration is edited directly and the lockstep check runs
- **THEN** the check fails, naming the file that diverged

#### Scenario: A new environment is covered without extending the check

- **WHEN** a new environment is declared, populated, and its configuration committed
- **THEN** the lockstep check covers the new file without any change to the check itself

### Requirement: Settings that must precede configuration loading are hand-maintained

Settings the tool manager reads before it loads any configuration file — including those enabling environment selection, lockfile enforcement, and provenance verification — SHALL live in a committed, hand-maintained file that the projection does not generate.

Such settings SHALL NOT be projected, because the projection is written on shell entry, which is after the first read in a fresh checkout. They SHALL be reviewed as changes to policy rather than recomputed from the declared tool set.

#### Scenario: Early-init settings apply in a fresh checkout

- **WHEN** a clean clone is made and the tool manager is run without first entering the devenv shell
- **THEN** environment selection, lockfile enforcement, and provenance verification are already in force

#### Scenario: Policy is not regenerated

- **WHEN** the projection is regenerated
- **THEN** the early-init settings file is not among the files it writes
