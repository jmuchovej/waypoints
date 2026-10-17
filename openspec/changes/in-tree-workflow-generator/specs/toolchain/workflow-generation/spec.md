## Purpose

Governs how the repository's CI definitions are produced and checked: the generator's library is part of the repository with its upstream provenance recorded and its dependencies the repository's to advance, lint suppressions are declared on the step they concern and emitted with it, and generated workflows pass the workflow linter in CI with no configuration-file ignores.

## ADDED Requirements

### Requirement: The generator's library is part of the repository

The library the workflow generator is built on SHALL be source in this repository, built as a path dependency, so that a change to it is an ordinary commit here. The upstream revision it derives from and the local delta against that revision SHALL be recorded beside the source, and there SHALL be a documented way to re-fetch upstream at a named revision so the local delta can be rebased onto it.

#### Scenario: A library change is one commit

- **WHEN** a contributor changes how a step or job is modelled
- **THEN** the change, the generator code using it, and the regenerated workflows land in a single commit in this repository, with no external repository, push, or revision bump involved

#### Scenario: Provenance is recorded

- **WHEN** the vendored library is inspected
- **THEN** the upstream repository and revision it was taken from, and a description of every local change against that revision, are recorded beside the source

#### Scenario: Upstream can be re-fetched

- **WHEN** a maintainer names a newer upstream revision
- **THEN** a documented recipe fetches that revision's source into the vendored location, leaving the local delta to be re-applied under version control's ordinary diff and conflict tooling

#### Scenario: The library is checked like first-party code

- **WHEN** the repository's lint, test, and format hooks run
- **THEN** the vendored library is covered by them

### Requirement: The generator's dependencies are the repository's to advance

No supply-chain advisory SHALL be carried as an accepted exception on the generator's behalf when the fix is a dependency change within this repository's manifests. The generator's serialization backend SHALL be a maintained crate. Where a chosen backend's versioning scheme makes every release a breaking change, that SHALL be recorded beside the source and its upgrades SHALL be review-gated rather than accepted automatically.

#### Scenario: No advisory is carried for the generator

- **WHEN** the supply-chain audit runs over the workspace
- **THEN** it passes with no ignored advisory whose subject is a dependency reached only through the workflow generator

#### Scenario: An unreachable pin is not an excuse

- **WHEN** a dependency of the generator's library carries an advisory whose fix is a version its own dependent pins away from
- **THEN** the dependent is a manifest in this repository, so the pin is changed rather than the advisory ignored

#### Scenario: A breaking-by-default dependency is flagged for review

- **WHEN** the generator depends on a crate whose release series makes every bump semver-breaking
- **THEN** that hazard is recorded beside the source and the dependency-update configuration treats its bumps as requiring review

### Requirement: A serializer change is proven not to change workflow meaning

Replacing or upgrading the generator's serialization backend MAY restyle the generated YAML — quoting, scalar style, or whitespace — and such a restyle SHALL be verified to leave every parsed value unchanged, by normalising the committed and regenerated workflows through a strict parser and comparing the results, rather than by byte comparison. A restyle SHALL be committed on its own, without a behavioural change in the same commit, after which byte comparison SHALL again be the standing check.

#### Scenario: A restyle is accepted on proof of equivalence

- **WHEN** the serialization backend changes and regeneration produces a textual diff in the committed workflows
- **THEN** each affected workflow's committed and regenerated forms normalise to identical parsed documents, and the diff is confined to serialization style

#### Scenario: A meaning change is caught

- **WHEN** a backend change alters a value's type or content — a quoted scalar becoming a number, boolean, or null
- **THEN** the normalised comparison differs and the change is rejected

#### Scenario: Byte equality is the standing check

- **WHEN** no serialization backend change is in flight
- **THEN** regeneration of the committed workflows is byte-identical

### Requirement: Lint suppressions are declared on the step they concern

A suppression of a workflow-linter finding SHALL be declared on the step it concerns, naming the rule and a reason, and SHALL be emitted into the generated workflow as a comment inside that step. No suppression SHALL be expressed at file or workflow scope. A declared suppression that cannot be placed — because the step has no stable identifier, or the identifier is not found in the rendered output — SHALL fail generation rather than be dropped.

#### Scenario: A suppressed step carries its comment

- **WHEN** a step declares a suppression and the workflows are generated
- **THEN** the generated step contains a linter-recognised comment naming the rule and reason, placed on a line within that step, and no other step is affected

#### Scenario: Unplaceable suppression fails loudly

- **WHEN** a step declares a suppression but has no identifier, or its identifier does not appear in the rendered output
- **THEN** generation fails naming the step, and no workflow file is written

#### Scenario: No file-scoped ignores

- **WHEN** the linter's configuration is inspected
- **THEN** it contains no rule ignore scoped to a workflow file

### Requirement: Generated workflows pass the workflow linter in CI

Every generated workflow SHALL pass the workflow linter with only the suppressions declared on its steps, and that check SHALL run on every pull request, so a suppression that stops matching or a finding that newly appears fails the pull request.

#### Scenario: A clean generation passes

- **WHEN** the workflows are regenerated from the committed generator and the linter runs over them in the pull-request workflow
- **THEN** the linter reports no findings

#### Scenario: A new cache in a publishing job is caught

- **WHEN** a cache-restoring action the linter knows is added to a job in a workflow that publishes, without a declared suppression
- **THEN** the pull-request workflow fails on the linter's finding

#### Scenario: A stale suppression is caught

- **WHEN** a declared suppression's anchor moves so the comment no longer falls within the finding's span
- **THEN** the pull-request workflow fails on the linter's finding

### Requirement: The generator does not re-implement the linter

The generator's own tests SHALL NOT detect caches, dangerous actions, or similar linter concerns by matching action names or step names. The generator's tests SHALL state positive invariants about the workflows it emits — which steps a job must contain, which environment it must declare, which values travel together — and SHALL leave recognition of third-party actions to the linter.

#### Scenario: Name matching is absent

- **WHEN** the generator's tests are inspected
- **THEN** no assertion rejects a step because its action or step name contains a substring

#### Scenario: Trust invariants are positive

- **WHEN** a generator test guards a trust boundary
- **THEN** it asserts the presence and ordering of the steps that establish the boundary — for example, that a compiler-cache wrapper appears only after the step resolving its credential, in a job declaring the matching environment
