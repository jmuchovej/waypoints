## ADDED Requirements

### Requirement: File-backed territories journal in WAL mode

A territory database backed by a file SHALL use write-ahead logging as its journal mode. WAL is what
permits pooled readers to run concurrently with the single writer, and it is the journal mode every
other durability statement in this section presumes.

An in-memory territory has no on-disk journal; journal mode is not meaningful for it and no journal
guarantee applies.

#### Scenario: Journal mode of an open file-backed territory

- **WHEN** a file-backed territory is open and its effective journal mode is read back from a live
  connection
- **THEN** the reported mode is WAL

#### Scenario: Journal mode of an in-memory territory

- **WHEN** an in-memory territory is open
- **THEN** no journal-mode guarantee is asserted, and durability operations that would act on a
  journal succeed as no-ops rather than failing

### Requirement: Committed writes are synchronously flushed

Every connection to a territory SHALL operate at the highest synchronous level the engine offers
(`FULL`), meaning the journal is flushed to the operating system and synced before a write is
reported as committed. This level SHALL be set explicitly by the storage layer and SHALL NOT be
inherited from an engine default.

A caller MAY lower the synchronous level for a specific database through explicit configuration; no
default path does so.

#### Scenario: Synchronous level of a default-configured territory

- **WHEN** a territory is opened with the default configuration and its effective synchronous level
  is read back from a live connection
- **THEN** the reported level is `FULL`

#### Scenario: Engine default changes underneath

- **WHEN** the storage engine's own default synchronous level differs from `FULL`
- **THEN** the territory's effective level is still `FULL`, because the storage layer sets it
  explicitly rather than relying on the engine's default

### Requirement: Durability configuration is verified at open and surfaced, not assumed

Opening a territory SHALL verify that the durability pragmas took effect and SHALL report a
distinguishable, structured mismatch when journal mode or synchronous level does not read back as
required. Configuration that silently fails to apply is treated as a reportable fault, not as a
warning to be swallowed.

The open SHALL NOT fail. A user whose notes are on disk must still be able to reach them; refusing to
open converts a durability degradation into total data inaccessibility, which is the worse outcome.

The mismatch SHALL be surfaced to the user rather than only logged, and the surface SHALL offer
whatever remediation is available (for example, retrying the pragma, or reopening the territory) and
SHALL offer to file a pre-filled bug report carrying the expected and effective values, the engine
version, and the platform. A durability mismatch is expected to be rare; when it does occur it
erodes trust in the storage layer, so it must be visible and actionable rather than silent.

Pragmas supplied by the caller beyond the durability pair remain best-effort: a rejection is
reported diagnostically and does not surface to the user.

#### Scenario: Durability pragma does not take effect

- **WHEN** a file-backed territory is opened and the engine does not report WAL journal mode, or does
  not report `FULL` synchronous
- **THEN** a usable database handle is returned, and the mismatch is reported as a structured fault
  naming the pragma, its required value, and its effective value

#### Scenario: User is offered remediation and a bug report

- **WHEN** a durability mismatch is reported at open
- **THEN** the user is shown the degradation, offered the available remediation, and offered a
  pre-filled bug report containing the expected value, effective value, engine version, and platform

#### Scenario: Caller-supplied pragma is rejected

- **WHEN** a territory is opened with a caller-supplied pragma the engine rejects
- **THEN** the rejection is reported diagnostically and the open succeeds

### Requirement: Durability configuration is gated before release

The durability pragmas SHALL be asserted by tests that run against the real engine, so that a pragma
which does not take effect is caught before a release is cut rather than in production.

Because the storage engine is pre-v1, engine upgrades SHALL be deliberately pinned, and the
durability assertions SHALL be re-run and reviewed as a precondition of accepting any upgrade. The
engine's per-connection default is explicitly not relied upon: the required values are set
explicitly and read back, so a change to an upstream default is caught by a failing test rather than
by silent behavioural drift.

#### Scenario: Engine upgrade changes an effective durability default

- **WHEN** the pinned storage engine is upgraded and its effective journal mode or synchronous
  default changes
- **THEN** the durability assertion tests fail, and the upgrade is not accepted until the
  configuration is corrected

### Requirement: Write-ahead log growth is bounded

The write-ahead log of an open territory SHALL be checkpointed automatically by the engine as it
grows, without the application scheduling checkpoints. Unbounded WAL growth is not an accepted
operating state.

#### Scenario: Sustained writing without explicit flushes

- **WHEN** a territory receives writes continuously and the application never requests an explicit
  flush or checkpoint
- **THEN** the write-ahead log is checkpointed by the engine and does not grow without bound

### Requirement: Explicit flushing is available and is caller policy

The storage layer SHALL expose an explicit flush operation with selectable behaviour — write dirty
pages out, or checkpoint the write-ahead log passively, fully, or with a restart. Choosing _when_ to
flush (idle timers, territory close, shutdown hooks) is the calling application's policy; the
storage layer SHALL NOT impose a flush schedule of its own.

Flushing is an optimisation for log size and reopen cost. It is not what makes a committed write
durable — that is established at commit.

#### Scenario: Explicit flush on a populated territory

- **WHEN** the application requests a flush in any of the available modes
- **THEN** the operation completes successfully and the territory remains readable and consistent

#### Scenario: Explicit flush on an in-memory territory

- **WHEN** the application requests a flush on an in-memory territory
- **THEN** the operation succeeds as a no-op

### Requirement: Closing a territory releases connections without checkpointing

Closing a territory SHALL release its connections and refuse subsequent operations. Closing SHALL
NOT be relied upon to checkpoint or truncate the write-ahead log; an application that wants a
compact log at close requests a flush before closing.

This is stated explicitly because the opposite is a natural assumption. A territory closed without a
flush is fully durable and reopens correctly — it simply reopens by replaying a log that was never
truncated.

#### Scenario: Operations after close

- **WHEN** an operation is attempted on a closed territory
- **THEN** it fails with a "closed" error

#### Scenario: Reopening a territory closed without a flush

- **WHEN** a territory is closed without an explicit flush and then reopened
- **THEN** every committed write is present and the database is not corrupt

### Requirement: Committed writes survive abrupt process termination

When the writing process is terminated abruptly — killed without unwinding, with no opportunity to
close the database or flush — the territory SHALL reopen without corruption, and every write that
was acknowledged as committed before termination SHALL be present after reopen.

#### Scenario: Kill during a single in-flight insert

- **WHEN** a process is killed without warning while a single insert is in flight, after earlier
  inserts were acknowledged
- **THEN** the territory reopens successfully, reports no integrity errors, and contains every
  previously acknowledged row

#### Scenario: Kill during an explicit multi-row transaction

- **WHEN** a process is killed without warning partway through an explicit transaction that has begun
  but not committed
- **THEN** the territory reopens successfully, reports no integrity errors, contains every row
  committed before that transaction began, and contains none of the uncommitted transaction's rows

#### Scenario: Kill while idle with a populated territory

- **WHEN** a process holding an open, populated territory is killed without warning while performing
  no write
- **THEN** the territory reopens successfully, reports no integrity errors, and its full contents are
  intact

#### Scenario: Reopen after abrupt termination is a normal open

- **WHEN** a territory left behind by a killed process is reopened
- **THEN** the open succeeds through the ordinary open path, with recovery handled by the engine, and
  no repair step is required of the application

### Requirement: Partially applied transactions are never visible

An explicit transaction SHALL be atomic across abnormal termination: after recovery, either all of
its writes are present or none are. A transaction interrupted before commit SHALL leave no partial
rows behind.

#### Scenario: Interrupted multi-row insert

- **WHEN** a multi-row insert that runs inside one transaction is interrupted by process termination
  before it commits
- **THEN** no row from that insert is present after reopen

### Requirement: The stated durability guarantee has an explicit boundary

The durability guarantees in this section SHALL be understood as covering the failure classes named
here, and no others. The distinction between what is verified by test and what is asserted by
configuration is part of the contract.

- **Process kill / abrupt close** — _verified_. Termination of the writing process without cleanup is
  exercised by tests against the real engine.
- **Operating-system crash** — _asserted, not verified_. Synchronous `FULL` under WAL means a
  committed write has been synced before acknowledgement, so an OS crash is expected to preserve
  acknowledged writes. No test in this repository establishes it.
- **Power loss** — _asserted, not verified_, and weaker still. It additionally depends on hardware and
  filesystem honouring sync, which the application cannot observe. No test in this repository
  simulates power loss, torn writes, or a sync that lies.
- **Media failure and filesystem corruption** — _out of scope_. No guarantee is made, and recovery
  from a damaged file is not a property of this section.

#### Scenario: Reading a durability claim from the tests

- **WHEN** the durability posture is assessed from the test suite
- **THEN** the suite demonstrates recovery from process termination only, and makes no assertion
  about power loss, operating-system crash, or media failure

## Decisions

### 2026-07-30 — Set `synchronous = FULL` explicitly rather than inheriting the engine default

**Decision.** The storage layer sets `synchronous = FULL` on every connection as part of its default
configuration, and verifies it read back at open.

**Rationale.** The engine currently defaults to `FULL` per connection, which is a deliberate
deviation from SQLite's `NORMAL` default under WAL. Depending on an upstream deviation — one an
engine bump could reverse for good reasons of its own — leaves the durability posture of every
territory outside our control and outside our tests.

**Rejected alternatives.**

- _Rely on the engine default and only assert it in a test._ The test would catch a regression, but
  only after an engine bump had already changed behaviour, and only if someone ran it before shipping.
  Setting the value makes the regression impossible rather than merely detectable.
- _Use `NORMAL` for write throughput._ Under WAL, `NORMAL` can lose the most recent committed
  transactions on OS crash or power loss. For a local-first application where the local file is the
  only copy of the user's data, losing acknowledged writes is not an acceptable trade for throughput.

### 2026-07-30 — Surfacing, not failing, when a durability pragma does not take effect

**Decision.** Journal mode and synchronous level are read back after the write connection opens. A
mismatch produces a structured, distinguishable fault that is **surfaced to the user** with available
remediation and a pre-filled bug report — but the open **succeeds**. Other caller-supplied pragmas
remain best-effort and are reported diagnostically only.

_Superseded an earlier draft of this decision that failed the open._ That was reversed on the
grounds below.

**Rationale.** A best-effort durability pragma is indistinguishable at runtime from a working one:
the database opens and serves reads and writes either way, so the failure is silent and the
consequence is data loss. It must therefore be detected and made visible. But refusing to open is
the wrong remedy — it converts a _degraded_ durability posture into _total inaccessibility of the
user's notes_, which is strictly worse than the condition it protests. The user's data is on disk
and readable; the correct response is to let them reach it while telling them plainly that the
storage layer is not behaving as guaranteed.

Visibility is the actual control. A mismatch should be rare, and precisely because it is rare it
erodes trust in the backend when it happens — so it warrants a user-facing surface and a one-click
report, not a log line. Detection also belongs earlier: the release gate (see the requirement above)
is what should keep this from reaching a user at all.

**Rejected alternatives.**

- _Fail the open._ Trades a recoverable degradation for guaranteed data inaccessibility. Wrong
  direction for a local-first application where the local file is the user's only copy.
- _Keep all pragmas best-effort and log louder._ Log lines are not a control; the process that most
  needs to see them is a desktop application whose stderr nobody reads.
- _Make every pragma fatal._ Caller-supplied pragmas are tuning knobs whose rejection is a legitimate
  engine-capability difference, not a correctness failure.

### 2026-07-30 — Scope crash testing to process termination, and say so

**Decision.** Crash-recovery tests cover abrupt process termination only. The spec names OS crash and
power loss as asserted-but-unverified, rather than folding them into the tested guarantee.

**Rationale.** Killing a process exercises the engine's recovery path honestly and runs in CI. Power
loss cannot be simulated in CI without a fault-injecting filesystem or virtual machine harness, and a
guarantee the tests do not establish is worse than an acknowledged gap — it invites reliance on
something unproven.

**Rejected alternatives.**

- _Claim power-loss durability on the strength of `synchronous = FULL`._ The configuration is
  necessary but not sufficient; hardware and filesystem behaviour are unobservable from the
  application.
- _Build a fault-injection harness to simulate power loss._ Disproportionate for the risk being
  managed, and it would test the engine's storage layer rather than this application's use of it.

### 2026-07-30 — Closing does not checkpoint, and the spec says so rather than changing it

**Decision.** Close semantics are documented as "release connections, no checkpoint" rather than
changed to checkpoint on close.

**Rationale.** Committed frames are already synced, so not checkpointing at close costs reopen time
and log size, not durability. Automatic checkpointing already bounds log growth during operation.
The application already has an explicit flush it can call at territory close, which keeps the _when_
in the application's hands where it belongs.

**Rejected alternatives.**

- _Checkpoint inside close._ Makes close a potentially long, blocking operation at exactly the moment
  an application is shutting down, in exchange for a benefit the caller can already opt into.
