## Purpose

Governs the lockfiles that pin the toolchain for consumers outside the devenv shell: what a lockfile must cover to be considered complete, when it may be written, and where an incomplete one is required to fail. Exists because the tool manager reports success on a lockfile it only partly wrote, which turns a transient network failure on one machine into a silent toolchain change everywhere else.

## ADDED Requirements

### Requirement: Every projected configuration has a committed lockfile

Each committed configuration the projection emits SHALL have a corresponding committed lockfile, and every such lockfile SHALL be covered by the generated-file lockstep check.

An uncovered lockfile is where drift accumulates unobserved, so the covered set SHALL be derived from the generator rather than listed by hand.

#### Scenario: Each environment is locked

- **WHEN** an environment is declared and populated
- **THEN** a lockfile for it is committed alongside its configuration, and the lockstep check covers both

### Requirement: A lockfile is complete only if it covers every tool on every target platform

Lockfile verification SHALL compare each configuration's declared tools against the lockfile's entries across the full set of target platforms, and SHALL fail on any missing pair.

Verification SHALL NOT be satisfied by tool resolution alone. A lockfile missing most of its platform entries still resolves every tool on the platform that wrote it, so a resolution check reports success on a lockfile that is unusable everywhere else.

Backends that legitimately produce no per-platform entries SHALL be exempted by explicit, named declaration. An unexplained absence SHALL be a failure, not a skip.

#### Scenario: Truncated lockfile is rejected

- **WHEN** a lockfile is missing platform entries for a tool that should carry them
- **THEN** verification fails and names the missing tool and platform pairs

#### Scenario: Resolution alone does not pass

- **WHEN** a lockfile resolves every declared tool on the current platform but omits entries for other target platforms
- **THEN** verification fails rather than reporting the toolchain as locked

#### Scenario: Exemptions are declared, not inferred

- **WHEN** a tool's backend produces no per-platform entries and is not in the exemption list
- **THEN** verification fails, rather than silently treating the absence as expected

### Requirement: Entering the shell verifies lockfiles; it never regenerates them

Entering the devenv shell SHALL verify the committed lockfiles without network access and without writing to them. Regeneration SHALL be an explicit, separately invoked action.

Automatic regeneration on shell entry makes a network failure indistinguishable from success: the tool manager writes what it managed to fetch and exits zero, so a throttled run silently replaces a complete lockfile with a partial one. Verification on entry instead fails on the machine that would have caused the damage.

#### Scenario: Shell entry is read-only

- **WHEN** the devenv shell is entered twice in succession
- **THEN** no lockfile is modified and no network request is made on either entry

#### Scenario: An incomplete committed lockfile fails at the shell, not in CI

- **WHEN** a lockfile missing entries is committed and a contributor enters the shell
- **THEN** entry fails with the coverage error, before the lockfile reaches a CI runner

### Requirement: Regeneration is authenticated and refuses to degrade

The explicit regeneration action SHALL regenerate every environment's lockfile in one run, and SHALL require credentials for the registries it queries.

Where credentials are absent, regeneration SHALL fail with a message naming what is missing, and SHALL NOT proceed unauthenticated. Unauthenticated regeneration is rate-limited well below the size of this tool set, and its failure mode is a short lockfile written with a success exit code.

#### Scenario: Missing credentials stop the run

- **WHEN** regeneration is invoked with no registry credentials available
- **THEN** it fails naming the missing credential, and no lockfile is written

#### Scenario: All environments regenerate together

- **WHEN** regeneration is invoked
- **THEN** every environment's lockfile is rewritten in that run, leaving none stale relative to the others

### Requirement: Installation enforces the lockfile and verifies provenance

Consumers installing from a projected configuration SHALL be required to install exactly what the lockfile pins, and SHALL verify the provenance of artifacts whose backends publish it.

A tool whose backend declares a signing identity SHALL be verified against the workflow that actually signed it, not merely against the presence of a valid signature. A registry entry that declares no signer identity accepts any valid signature, including one from an unrelated workflow, and so provides no provenance guarantee.

#### Scenario: Unlocked install is refused

- **WHEN** a consumer installs a tool that the active configuration declares but the lockfile does not pin
- **THEN** installation fails rather than resolving the version afresh

#### Scenario: Wrong signer is rejected

- **WHEN** an artifact carries a valid signature from a workflow other than the one the tool's declaration names
- **THEN** verification fails and the tool is not installed
