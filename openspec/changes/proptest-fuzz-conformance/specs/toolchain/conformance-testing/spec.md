## Purpose

How generated-input testing behaves in this repository: property tests over
generated values that shrink and remember their failures, and coverage-guided
fuzzing of every parser entry point, so a format meant to last is checked by
tooling that finds and keeps the cases a hand-written generator misses.

## ADDED Requirements

### Requirement: A failing generated case shrinks and persists

A test over generated inputs SHALL reduce any failing input to a minimal
failing input before reporting it, and SHALL record a reproducer for that
input in a file committed to the repository. Every subsequent run of that
test SHALL replay the recorded reproducers before generating new inputs, so a
fixed bug cannot return unnoticed.

#### Scenario: A failure is minimal

- **WHEN** a generated store fails a round-trip property because one table
  cell is wrongly escaped
- **THEN** the reported failing store contains that cell and nothing the
  failure does not depend on

#### Scenario: A failure is remembered

- **WHEN** a generated case fails and the fix is committed
- **THEN** the reproducer file in the repository replays that case on the
  next test run, and a regression of the fix fails the test deterministically

### Requirement: One generator produces only domain-valid values

Random values of the data model used by tests and fuzzers SHALL come from one
generator per type, and that generator SHALL produce only values the domain
admits: a mention's target is an id, a colour-bearing mark names a colour,
a code block's language is a token, a reading fits the kind of the feature it
is read for. Property tests and fuzz targets SHALL share these generators
rather than each describing validity again.

#### Scenario: Generated values survive the round trip for the right reason

- **WHEN** a generated rich text is emitted and parsed back
- **THEN** any inequality is a dialect fault, never an input the domain would
  have rejected

#### Scenario: Validity is stated once

- **WHEN** a new mark kind with constrained data is added to the data model
- **THEN** its validity rule is written in one generator and both the
  property tests and the fuzz targets begin producing it

### Requirement: Every parser entry point has a fuzz target

Each public parse entry of the dialect and the Bundle (inline text, a single
block, a document region) SHALL have a coverage-guided fuzz target that feeds
arbitrary bytes and requires the parser to return a value or a refusal
without panicking, and the inline and block layers SHALL additionally have a
target that feeds generated domain values through emit and parse and requires
equality. Targets SHALL start from a seed corpus drawn from the committed
`getting-started` Bundle and from emitter output, so the fuzzer mutates valid
inputs rather than discovering the grammar from nothing.

#### Scenario: A crash becomes a test

- **WHEN** a fuzz target finds an input that panics the parser
- **THEN** the input is minimized and committed as a unit test in the crate
  that owns the fault, and the fuzz artifact itself is not committed

#### Scenario: Seeded, not blind

- **WHEN** a fuzz target starts
- **THEN** its initial corpus contains every block line of the committed
  Bundle and a sample of emitter output

### Requirement: Fuzz targets compile on every commit

The fuzz targets SHALL type-check as part of the per-commit hooks on the
workspace's pinned toolchain, so a change to a parser signature cannot leave
a target that no longer builds. Running a fuzzer SHALL be a single command and
SHALL NOT be part of the hooks.

#### Scenario: A signature change is caught

- **WHEN** a parse entry point changes its signature and a fuzz target is not
  updated
- **THEN** the per-commit hooks fail at the fuzz check

#### Scenario: One command fuzzes

- **WHEN** a developer runs the fuzz recipe with a target name
- **THEN** the target builds with the fuzzing toolchain and runs against its
  seed corpus, with no other setup

### Requirement: Case count is one variable

The number of generated cases a property test runs SHALL default to a count
that keeps the workspace test run fast, and SHALL be raised for a deep run by
setting one environment variable, with no code change.

#### Scenario: Deep run

- **WHEN** the case-count variable is set to ten thousand
- **THEN** every property test runs ten thousand generated cases plus its
  recorded reproducers
