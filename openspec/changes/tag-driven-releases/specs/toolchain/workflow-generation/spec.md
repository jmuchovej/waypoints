## ADDED Requirements

### Requirement: Shared setup prefixes are generated composite actions

A setup sequence that more than one job runs before its own work SHALL be a composite action the generator emits under the repository's actions directory, carrying the header every generated workflow carries, and each job SHALL run it as one step by the self-repository reference, after the job's own checkout. The generator's sweep and drift check SHALL cover the actions directory as they cover the workflows directory.

#### Scenario: A job's setup is one generated step

- **WHEN** a generated job is inspected
- **THEN** its step after the checkout is a self-repository reference to a generated action, and the job carries no bare step that the action performs

#### Scenario: A renamed action leaves no orphan

- **WHEN** an action is renamed in the generator and the workflows are regenerated
- **THEN** the file under the old name is removed, and the drift check run against a tree still holding it names it as generated with no source

#### Scenario: A checkout precedes the setup

- **WHEN** a job's steps are read in order
- **THEN** the job's own checkout, with the depth, tags, and ref that job needs, comes before the setup action, and no generated action performs a checkout

### Requirement: A generated action holds setup alone, and is audited as the job's steps

A generated composite action SHALL NOT contain a job's checkout, a step that brackets the job's work with a counterpart after it, or a step that holds a credential the job's environment grants. The generator's audits SHALL assert over a job's effective steps — its own, with each generated action's steps in place of the step that uses it, and a called workflow's jobs in place of the job that calls it — so no invariant is escaped by moving a step into an action or a job into a called workflow.

#### Scenario: An action's steps are audited as the job's

- **WHEN** a generator test asserts a per-step invariant over a job
- **THEN** the steps of every generated action the job uses are included where its reference stands, and the invariant holds or fails over the whole

#### Scenario: A called workflow's jobs are audited with the caller's

- **WHEN** a generator test asserts an invariant over a workflow that calls another
- **THEN** the called workflow's jobs are included where the calling job stands, and the invariant holds or fails over the whole

#### Scenario: A bracketing step stays in the job

- **WHEN** a job restores a snapshot before its work and saves it after, or starts a credentialed server its work compiles through
- **THEN** both halves are steps of the job itself, in the job's environment, and no generated action carries either

## MODIFIED Requirements

### Requirement: Lint suppressions are declared on the step they concern

A suppression of a workflow-linter finding SHALL be declared on the step it concerns, or on the job where the finding is the job's own — a key of the job rather than of any step — naming the rule and a reason, and SHALL be emitted into the generated workflow as a comment inside that step or on the job's line the finding spans. No suppression SHALL be expressed at file or workflow scope. A declared suppression that cannot be placed — because the step has no stable identifier, the identifier is not found in the rendered output, or the job carries no line the finding spans — SHALL fail generation rather than be dropped.

#### Scenario: A suppressed step carries its comment

- **WHEN** a step declares a suppression and the workflows are generated
- **THEN** the generated step contains a linter-recognized comment naming the rule and reason, placed on a line within that step, and no other step is affected

#### Scenario: Unplaceable suppression fails loudly

- **WHEN** a step declares a suppression but has no identifier, or its identifier does not appear in the rendered output
- **THEN** generation fails naming the step, and no workflow file is written

#### Scenario: No file-scoped ignores

- **WHEN** the linter's configuration is inspected
- **THEN** it contains no rule ignore scoped to a workflow file

#### Scenario: A job-scoped finding carries its comment

- **WHEN** a calling job inherits the caller's secrets and declares the suppression that inheritance earns, with the reason an environment secret cannot be passed by name
- **THEN** the generated job carries the linter-recognized comment on its `secrets` line, and a job declaring such a suppression without that line fails generation naming the job

### Requirement: Generated workflows pass the workflow linter in CI

Every generated workflow and every generated action definition SHALL pass the workflow linter with only the suppressions declared on their steps and jobs, and that check SHALL run on every pull request over the whole actions directory tree — workflows and action definitions alike — so a suppression that stops matching or a finding that newly appears fails the pull request. Where the linter cannot see into a generated action — a cache it would recognize in a job's own steps but not behind an action — the generator's own tests SHALL hold the invariant the linter would have held.

#### Scenario: A clean generation passes

- **WHEN** the workflows are regenerated from the committed generator and the linter runs over them in the pull-request workflow
- **THEN** the linter reports no findings

#### Scenario: A new cache in a publishing job is caught

- **WHEN** a cache-restoring action the linter knows is added to a job in a workflow that publishes, without a declared suppression
- **THEN** the pull-request workflow fails on the linter's finding

#### Scenario: A stale suppression is caught

- **WHEN** a declared suppression's anchor moves so the comment no longer falls within the finding's span
- **THEN** the pull-request workflow fails on the linter's finding

#### Scenario: A cache behind an action is held by the generator

- **WHEN** a job that declares an environment asks a generated setup action for a dependency cache the linter would flag in the job's own steps
- **THEN** a generator test refuses the job, naming it, before any workflow is written

#### Scenario: Action definitions are audited with the workflows

- **WHEN** the pull-request workflow runs the linter
- **THEN** it is given the actions directory tree, so every generated action definition is audited beside every workflow, and a local reference in either that is not the self-repository form is a finding
