## Context

See `proposal.md` — Why, for motivation and the three holes found during investigation. What the
implementation has to work with:

- The pool opens one write connection and lazily grows up to a ceiling of read connections, all from
  a single engine handle. Pragmas are applied per connection at creation time, and the engine's
  synchronous level is per-connection state.
- Pragma application today is fire-and-forget: rejections print to stderr and the open proceeds.
- The engine's `PRAGMA synchronous` accepts SQLite's spellings and its per-connection default is
  the strictest level — a deliberate upstream deviation from SQLite, which we are choosing not to
  depend on.
- Automatic WAL checkpointing is enabled by default with a frame threshold, so log growth is already
  bounded during operation. This is a fact to assert, not a gap to fix.
- The engine's `Connection::close()` performs a shutdown checkpoint; its `Drop` deliberately does
  not. The pool's close path drops connections, so no shutdown checkpoint happens. Committed frames
  are already synced, so this is a documentation defect rather than a durability defect.
- No test in the crate reads any pragma back. The existing tests reach raw SQL through the
  transaction types, and roll their own temp directories out of `std::env::temp_dir()` rather than
  taking a dependency on a temp-file crate.

Binding constraints: persistence tests run against the real engine and never assert SQL strings;
every commit must leave clippy, `cargo test`, and treefmt green, which means the crash tests run in
the per-commit hook and must be fast and non-flaky.

## Goals / Non-Goals

**Goals:**

- Make the durability configuration an enforced, readable property rather than an assumed one.
- Give crash-recovery tests a real killed process, on all three CI platforms, with no new
  dependencies and no sleep-based synchronization.
- Keep assertions race-tolerant, so a test can never fail because a kill landed a few microseconds
  earlier or later than the previous run.

**Non-Goals:**

- No fault injection below the engine (torn writes, lying `fsync`, filesystem-level failure). See
  the spec's guarantee-boundary requirement.
- No change to how or when the application flushes; scheduling stays caller policy.
- No repair or salvage path for an already-corrupt database.
- No new public surface beyond the two small readback APIs described below.

## Decisions

### Durability pragmas become a verified pair, everything else stays best-effort

`Config` grows an explicit synchronous setting alongside the existing `wal` flag, defaulting to
`FULL`. After the write connection is created, the pool reads both `PRAGMA journal_mode` and
`PRAGMA synchronous` back and reports a structured mismatch through a new fault type when either
does not match what was requested. **The open still succeeds.** Caller-supplied pragmas keep their
current best-effort contract.

_Why:_ the failure mode being closed is a silent one — a database that opens and works while
quietly not being durable. Reading the value back from the connection that will do the writing is
the only evidence that the pragma took.

_Why not fail the open:_ refusing to open turns a degraded-but-usable database into an unreachable
one. The user's notes are on disk and readable; denying access to protest a pragma is a worse
outcome than the degradation. The mismatch is instead **surfaced to the user** with whatever
remediation exists and a pre-filled bug report (expected value, effective value, engine version,
platform). Rare faults that erode trust in the storage layer need to be visible and actionable, not
silent — and not weaponised against the user's access to their own data.

_Where detection really belongs:_ the release gate. The durability assertions run against the real
engine before a release is cut, and are re-run as a precondition of accepting any storage-engine
upgrade (the engine is pre-v1 and deliberately pinned). A pragma that does not take effect should
never reach a user; the runtime surface is the backstop, not the primary control.

_Alternatives rejected:_ failing the open (trades recoverable degradation for guaranteed
inaccessibility); asserting the pragmas only in a test with no runtime check (leaves the user with
no signal if it does slip through); making every pragma fatal (turns legitimate engine-capability
differences into open failures).

_Consequence worth naming:_ this converts a silent degrade into a **visible** one. A platform or
engine that cannot provide WAL plus `FULL` still opens the territory, but the user is told the
storage layer is operating below its guarantee, offered any available remediation, and offered a
one-click report. The cost is that a user _can_ keep working in a degraded posture — accepted
deliberately, because the alternative is denying them their notes. The release gate is what keeps
that path rare.

### Verify on the write connection only

The readback happens once, on the write connection, during open — not on each pooled read
connection as it is created lazily.

_Why:_ synchronous level governs what happens when a write is committed, and only the write
connection commits. Journal mode is a property of the database file, not of a connection. Verifying
per read connection would add a round trip to every lazy read-pool growth to re-prove a property
that cannot have changed.

### A small readback API rather than raw pragmas in tests

Two additive methods on the database handle:

- one returning the effective journal mode and synchronous level, read from a live connection — the
  same code path open uses for its check;
- one running an integrity check and reporting the engine's findings.

_Why:_ the pool needs the first internally regardless, so exposing it costs nothing and gives the
regression guard a first-class assertion instead of hand-parsed pragma rows in a test. The second is
what a crash test needs, and it is also legitimately product-facing: an application that has just
reopened a territory left behind by a killed process has a real reason to ask whether the file is
sound. Both are read-only and small.

_Alternatives rejected:_ raw `PRAGMA` strings through the transaction types in the tests. It would
not violate the "no string-asserted SQL" rule — the pragma genuinely executes — but it duplicates
pragma-row parsing into the test file and leaves the shipped code with no way to answer a question
the application will eventually want to ask.

### Crash victim as a dev-only `[[bin]]` in the bedrock crate

A small binary target in the same package, spawned by the integration tests through the
`CARGO_BIN_EXE_<name>` environment variable Cargo provides. It takes a database path and a scenario
name, drives writes against a real file-backed territory, and prints single-line progress markers.

_Why:_ `CARGO_BIN_EXE_` is only populated for `[[bin]]` targets of the same package, and it is the
one mechanism that gives the test a correctly-built binary path with no extra dependency and no
build scripting. The package is `publish = false`, so an extra tiny bin target costs nothing beyond
its own compile.

_Alternatives rejected:_ forking in-process (unsound alongside a running tokio runtime, and
Unix-only); an `examples/` target plus a crate to locate and build it (a new dependency for a path
lookup); a separate workspace member crate (a whole package for one file).

### Marker-driven kill, never sleep-driven

The victim writes a marker line and explicitly flushes it after each meaningful step. The test reads
the child's stdout line by line and kills the child when it sees the marker for the scenario's kill
point. A reader thread plus a receive-with-timeout keeps a missing marker from hanging the suite,
and a guard type kills the child on drop so a failing assertion cannot leak a process.

_Why:_ sleeping to "get somewhere mid-write" is the standard way these tests become flaky under
load, which is exactly when CI runs them. Markers make the kill point causal rather than temporal.

_Detail that matters:_ the child's stdout is a pipe, not a terminal, so it is block-buffered. Every
marker must be explicitly flushed or the test will block forever waiting for output the child has
already produced.

### Assert invariants, not exact counts

Kill timing is inherently racy relative to an in-flight write, so each scenario asserts a property
that holds either side of the race:

| Scenario                               | Kill point                                                    | Assertion                                                                                                         |
| -------------------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| In-flight single insert                | after an acknowledged row, while the next insert is in flight | integrity clean; every acknowledged row present; a row beyond the last acknowledgement is permitted, not required |
| Held-open explicit transaction         | after `BEGIN IMMEDIATE` and several uncommitted inserts       | integrity clean; every pre-transaction row present; no row from the uncommitted transaction present               |
| Batch insert through the multi-row API | immediately after the batch starts                            | integrity clean; the batch is all-present or all-absent — never partial                                           |
| Idle with a populated territory        | after the last acknowledgement, doing nothing                 | integrity clean; contents exactly as acknowledged                                                                 |

_Why:_ the only assertion that would be race-sensitive is an exact row count on an interrupted
write, and it is also the assertion nobody actually cares about. "No acknowledged write was lost"
and "no uncommitted write was visible" are the guarantees, and both are stable under any kill
timing.

The held-open-transaction scenario is deterministic by construction — the transaction never commits,
so its rows must never appear. The batch scenario is deliberately racy and paired with an
all-or-nothing assertion, which is what exercises the `BEGIN IMMEDIATE` path the multi-row insert
uses internally.

### Temp directories follow the existing hand-rolled pattern

Per-test unique directories under the system temp dir, keyed by process id and test name, cleaned on
success.

_Why:_ the crate's existing tests already do this and adding a temp-file dependency for parity with
them is not worth a dependency. Per-test uniqueness matters because the test harness runs tests
concurrently and each victim needs its own file.

## Risks / Trade-offs

- **A killed process leaves sidecar files (`-wal`, `-shm`) behind, and reopen depends on the engine
  recovering from them.** → That dependency is the thing under test. If recovery turns out to be
  weak on some platform, the test failing is the correct outcome and a genuine finding, not a test
  defect to paper over with a skip.
- **Windows termination semantics differ from a Unix signal.** → Both the Unix and Windows paths of
  the standard kill primitive terminate without giving the child a chance to clean up, which is what
  the scenario requires. The risk is not the kill; it is whether the engine's recovery is exercised
  as thoroughly on Windows, and the full matrix only runs on merge. Accepted: a platform-specific
  recovery failure surfaces at merge rather than at commit.
- **Spawning processes inside the per-commit test hook costs wall-clock time.** → Row counts stay in
  the low hundreds and there are no sleeps, so each scenario is dominated by process startup. Keep
  the scenario count at the four above; if the hook budget becomes a problem, the mitigation is
  fewer scenarios, not sleep-shortened ones.
- **Failing the open on a durability mismatch is a hard behaviour change.** → It only triggers where
  the previous behaviour was to run non-durably in silence. The error is distinguishable, so a
  caller that genuinely wants a weaker configuration can ask for one explicitly.
- **Setting `synchronous = FULL` explicitly where the engine already defaulted to it means no
  measurable change today, so the guard could be mistaken for dead code.** → The regression guard
  test is what makes the intent legible; the spec decision entry records why the explicit set exists
  even though it is currently a no-op against this engine version.
- **Correcting the two false comments changes no behaviour and is easy to lose in review.** → They
  are separate, named tasks rather than drive-by edits, precisely because the false beliefs they
  encode are what motivated this change.

## Migration Plan

Purely additive: no schema change, no data migration, no `user_version` bump, and no change to how
an existing territory file is laid out. An existing territory opens under the new code exactly as
before, with the durability readback as the only new step in the open path.

Rollback is reverting the commits; nothing written under the new code is unreadable by the old code.

## Open Questions

- Whether the application's territory-close flush should move from a passive checkpoint to a full or
  restarting one, now that closing is confirmed not to checkpoint. This affects reopen cost and
  on-disk log size, not durability, and can be decided from measurement later without touching the
  specs, the approach, or the task breakdown.
