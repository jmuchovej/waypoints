# toolchain/lockfile-integrity Specification

## Purpose

Governs the lockfiles that pin the toolchain for consumers outside the devenv shell: which configurations must have one, when one may be written, and where an incomplete one is required to fail. Exists because the tool manager reports success on a lockfile it only partly wrote, which turns a transient network failure on one machine into a silent toolchain change everywhere else.

## Requirements

### Requirement: Every projected configuration has a committed lockfile

Each committed configuration the projection emits SHALL have a corresponding committed lockfile, and every such lockfile SHALL be covered by the generated-file lockstep check.

An uncovered lockfile is where drift accumulates unobserved, so the covered set SHALL be derived from the generator rather than listed by hand.

#### Scenario: Each environment is locked

- **WHEN** an environment is declared
- **THEN** a lockfile for it is committed alongside its configuration, and the lockstep check covers both — including an environment whose tools all sit in the base configuration, which still emits both files so a consumer's selection does not depend on what the lane currently holds

### Requirement: Lockfile completeness is enforced at installation, not by a local check

Completeness SHALL be enforced where an incomplete lockfile actually causes harm: the installing consumer. Installation SHALL be required to install exactly what the lockfile pins, so a tool the lockfile does not cover on the installing platform fails there rather than resolving afresh.

A local per-platform coverage check SHALL NOT be required. Such a check can only report, before a push, that a platform the author is not on is short — and it cannot act on that finding, because refreshing a lockfile extends only the platforms it already carries. Widening it is a separate, explicitly targeted operation. Against that narrow benefit it needs an expected-platform set and a per-tool exemption table, neither derivable: the platforms the tool manager can lock are not the platforms this repository targets, and the Rust toolchain file enumerates build targets rather than hosts.

Drift in a committed lockfile SHALL instead be caught by the generated-file lockstep check, which fails on any lockfile difference.

#### Scenario: A lockfile short on the installing platform fails there

- **WHEN** a consumer installs on a platform for which the lockfile carries no entry for a declared tool
- **THEN** installation fails on that platform rather than resolving the version afresh

#### Scenario: Shell entry does not assess coverage

- **WHEN** a committed lockfile is missing platform entries for a platform other than the current one
- **THEN** nothing local fails — the gap surfaces at an install on the affected platform, and no expected-platform set or exemption table is maintained to anticipate it

#### Scenario: Lockfile drift is caught by the lockstep check

- **WHEN** a committed lockfile differs from what regeneration would produce
- **THEN** the generated-file lockstep check fails

### Requirement: Entering the shell neither writes nor reads lockfiles

Entering the devenv shell SHALL NOT write to a lockfile, and SHALL NOT make a network request on behalf of one. Regeneration SHALL be an explicit, separately invoked action.

Automatic regeneration on shell entry makes a network failure indistinguishable from success: the tool manager writes what it managed to fetch and exits zero, so a throttled run silently replaces a complete lockfile with a partial one — unattended, on every entry, with the damage surfacing at a CI install far from the machine that caused it.

Entry SHALL NOT verify lockfiles either. Verification on entry was considered and rejected with the coverage check above: it is the same check, paid on every entry, and its findings are not actionable at that moment.

#### Scenario: Shell entry is read-only

- **WHEN** the devenv shell is entered twice in succession
- **THEN** no lockfile is modified and no network request is made on either entry

#### Scenario: No task writes a lockfile

- **WHEN** the tasks that run on shell entry are enumerated
- **THEN** none of them writes a lockfile

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

Verification SHALL be enforced rather than advisory: an artifact whose attestation is absent or does not verify SHALL NOT be installed.

Pinning a tool to a _named signer_ is a stronger guarantee than attestation, and is deliberately not required here. It was carried by the one tool whose registry entry declared no signer — so any valid signature satisfied it, including one from an unrelated workflow — and that tool left the projection with `secretspec-onepassword`. Should a tool need it again, the mechanism is a per-tool backend option, and this requirement is where the obligation to name the signer belongs.

#### Scenario: Unlocked install is refused

- **WHEN** a consumer installs a tool that the active configuration declares but the lockfile does not pin
- **THEN** installation fails rather than resolving the version afresh

#### Scenario: Attestations are verified, not assumed

- **WHEN** a consumer installs a tool from a backend that publishes artifact attestations
- **THEN** the attestation is verified before the tool is installed, and a failure to verify prevents installation
