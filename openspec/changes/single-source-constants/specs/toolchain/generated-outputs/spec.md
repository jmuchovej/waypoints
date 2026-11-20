## ADDED Requirements

### Requirement: Generated outputs are declared once, by generator

The tooling SHALL carry one ledger naming, per generator, the sources it reads and the outputs it owns. The lockstep check's trigger and its diff set, and the listing a recipe reads, SHALL derive from the ledger and SHALL hold no list of their own. An output a tool reports only at run time MAY be read at run time; every other output SHALL be a static entry of the ledger.

#### Scenario: A generator is added

- **WHEN** a new generator lands with its ledger entry
- **THEN** the check's trigger, its diff set, and the listing cover the new outputs with no other edit

#### Scenario: The check and the listing agree

- **WHEN** the set the lockstep check diffs is compared with what the listing prints
- **THEN** they are the same set

#### Scenario: A hand-edited output

- **WHEN** a file under a declared output is edited by hand and the lockstep check runs
- **THEN** the check fails naming the file

### Requirement: A generator holds to its declared outputs

A generator that computes its placements at run time SHALL refuse, before writing, to place or remove any path outside the outputs its ledger entry declares, naming the path. Outputs SHALL be declared as the roots a deletion is visible under, not only as the files a run writes.

#### Scenario: A placement outside the roots

- **WHEN** a generator's own manifest names an output no declared root covers
- **THEN** generation fails before any committed file changes, naming the path and the roots

#### Scenario: A removed output is visible

- **WHEN** a source no longer describes an output but the file is still committed
- **THEN** the check diffs the root and the stale file shows as a deletion

### Requirement: The tooling's shared names and paths have one definition

A path or name more than one module of the tooling reads — a branch, a bot identity, a manifest, a generated file — SHALL be defined once and imported by every reader; a second definition, or a literal spelled in a reader, SHALL be removed. A constant one module alone reads MAY stay beside that reader. A test fixture reproducing a file's content is not a reader.

#### Scenario: A renamed branch

- **WHEN** the bot's branch prefix changes
- **THEN** one definition changes and every workflow, planner, and task that names it follows

#### Scenario: The bot's identity

- **WHEN** the tooling signs a commit as the bot or names the bot as a pull request's author
- **THEN** both read the one definition of the bot's login and address
