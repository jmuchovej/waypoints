## 1. Durability configuration

- [x] 1.1 Add an explicit synchronous setting to `Config` in `crates/bedrock/src/config.rs`: a typed
      level (off / normal / full) with a builder setter, defaulting to full, documented as the
      crash-safety knob and as deliberately not inherited from the engine default.
- [x] 1.2 Apply the synchronous setting to every connection the pool creates in
      `crates/bedrock/src/pool.rs`, alongside the existing WAL pragma, before caller-supplied
      pragmas so a caller can still override deliberately.
- [x] 1.3 Add a `BedrockError` variant in `crates/bedrock/src/error.rs` for a durability pragma whose
      effective value does not match what was requested — carrying the pragma name, the requested
      value, and the effective value.

## 2. Verified open

- [x] 2.1 Add a durability readback helper in `crates/bedrock/src/durability.rs` that reads
      `PRAGMA journal_mode` and `PRAGMA synchronous` from a connection and returns the effective
      values as typed data (not strings passed through to callers).
- [x] 2.2 Call the readback from `Pool::open` after the write connection is created. When journal
      mode is not WAL on a file-backed database, or the synchronous level is not what `Config`
      requested, **return the handle successfully and attach a structured durability-mismatch fault**
      carrying the pragma, its required value, and its effective value. The open MUST NOT fail —
      denying access to readable notes is worse than the degradation — but the handle is
      **read-only**: refuse write operations with an error carrying the fault, and leave reads
      untouched. Skip the journal-mode half for `:memory:`.
- [x] 2.2a Surface the fault to the caller as retrievable state on `Database` (not a log line only),
      so the app layer can present it, offer available remediation, and offer a pre-filled bug report
      containing expected value, effective value, engine version or commit hash, and platform. The
      engine identifier is recorded at build time from the workspace's locked dependency resolution
      (the exact pin names the `turso` facade rather than the engine underneath it, and no useful
      version is exposed at runtime). Keep the fault informative enough for `cartograph` to later
      drive real resolution (for example, live-migrating to a fresh database) — that machinery is
      out of scope here, but this error is its input.
- [x] 2.3 Narrow the doc comment on `apply_pragma` so its best-effort contract explicitly covers
      caller-supplied pragmas only, and points at the verified pair as the exception.
- [x] 2.4 Expose the effective durability state as a small public read-only method on `Database` in
      `crates/bedrock/src/database.rs`, reusing the helper from 2.1 so open and the accessor share
      one code path.
- [x] 2.5 Add a public integrity-check method on `Database` that runs the engine's integrity check
      and reports its findings, for use after reopening a territory left by a killed process.
- [x] 2.6 Record the app-layer follow-up in `cartograph` as a written TODO, not an implementation:
      next to the `store::open` call in `crates/cartograph/src/runtime.rs` (~line 48), note that the
      durability fault retrievable from `Database` (2.2a) needs handling there — presenting the
      read-only degradation to the user, offering remediation and the pre-filled bug report, and
      eventually real resolution such as live-migrating the territory to a fresh database. Deferred
      deliberately: it needs UI surface and UI testing beyond this change's scope.

## 3. Documentation corrections

- [x] 3.1 Correct the `FlushMode::Fsync` doc comment in `crates/bedrock/src/durability.rs`: it writes
      dirty pages out to the log rather than performing a durability fsync, and it is not what makes
      a committed write durable.
- [x] 3.2 Correct the comment in `crates/store/src/tools.rs` that claims the engine flushes the log
      on close — closing releases connections and does not checkpoint, which is why the explicit
      flush at territory close exists.
- [x] 3.3 Document the close-without-checkpoint behaviour on `Database::close` so the next reader
      does not re-derive it.

## 4. Crash victim harness

- [x] 4.1 Add a `[[bin]]` target to `crates/bedrock/Cargo.toml` for the crash victim, with its source
      under `crates/bedrock/src/bin/`, and confirm the workspace still builds and lints clean with
      the extra target.
- [x] 4.2 Implement the victim: takes a database path and a scenario name as arguments, opens a
      file-backed territory with default config, ensures a small test collection, and dispatches to
      the scenario. Every progress marker is a single line on stdout followed by an explicit flush.
- [x] 4.3 Implement the `single-insert` scenario: insert rows one at a time, printing an
      acknowledgement marker after each successful insert, continuing indefinitely.
- [x] 4.4 Implement the `open-transaction` scenario: insert and acknowledge a baseline, then begin an
      explicit write transaction, insert several rows without committing, print the transaction-open
      marker, and park forever.
- [x] 4.5 Implement the `batch-insert` scenario: insert and acknowledge a baseline, print the
      batch-start marker, then issue one multi-row insert through the batch API so the kill lands
      inside its internal `BEGIN IMMEDIATE` transaction.
- [x] 4.6 Implement the `idle` scenario: insert and acknowledge a fixed population, print the idle
      marker, and park forever without writing.

## 5. Crash-recovery tests

- [x] 5.1 Add `crates/bedrock/tests/crash_recovery.rs` with the shared harness: spawn the victim with
      piped stdout, read markers on a reader thread with a receive timeout so a missing marker fails
      instead of hanging, kill on the target marker, and a guard type that kills the child on drop so
      a failing assertion never leaks a process. Per-test temp directories under the system temp dir,
      keyed by process id **and test name** (extending the existing pool test's pid-only pattern —
      tests run as parallel threads in one process, so the pid alone is shared) — no new dependency.
- [x] 5.2 Test the in-flight single insert: kill after an acknowledged row while the next insert is
      in flight; reopen and assert integrity is clean and every acknowledged row is present, tolerating
      one extra row beyond the last acknowledgement.
- [x] 5.3 Test the held-open explicit transaction: kill on the transaction-open marker; reopen and
      assert integrity is clean, the full baseline is present, and no row from the uncommitted
      transaction is present.
- [x] 5.4 Test the interrupted batch insert: kill on the batch-start marker; reopen and assert
      integrity is clean and the batch is entirely present or entirely absent, never partial.
- [x] 5.5 Test reopen-after-kill on a populated idle territory: kill on the idle marker; reopen and
      assert integrity is clean and the contents match the acknowledged population exactly.
- [x] 5.6 Assert across the scenarios that reopening a killed territory succeeds through the ordinary
      open path with no repair step, and that the sidecar log files left behind do not prevent it.

## 6. Regression guard

- [x] 6.1 Add a test that opens a file-backed territory with default config and asserts, from the
      live connection, that the effective journal mode is WAL and the effective synchronous level is
      full — so weakening either default fails the test rather than shipping.
- [x] 6.2 Add a test that an explicitly weakened synchronous level is honoured and reported as such,
      proving the guard asserts effective state rather than a hardcoded expectation.
- [x] 6.3 Add a test that an in-memory territory opens successfully with no journal-mode guarantee and
      that flush operations on it succeed as no-ops.
- [x] 6.4 Add a test that a caller-supplied pragma the engine rejects does not fail the open, keeping
      the best-effort contract for non-durability pragmas covered.
- [x] 6.5 Add a test that a durability mismatch **still yields a readable handle**: reads succeed,
      write operations are refused with an error carrying the durability fault, and the structured
      fault is retrievable from the handle — the opens-read-only contract must itself be guarded, or
      a later refactor will quietly reintroduce either a hard open failure or silent writability.

## 6b. Release gate (storage engine is pre-v1 and deliberately pinned)

- [x] 6b.1 Document, next to the engine's pin, that the durability assertion tests are a precondition
      for accepting any storage-engine upgrade — an upstream change to a default journal mode or
      synchronous level must fail a test, never drift in silently.
- [x] 6b.2 Confirm the durability assertions run in the same suite as the standard pre-release check
      (`prek` / `cargo test`), so no separate opt-in step stands between a pragma regression and the
      person cutting the release.

## 7. Close out

- [x] 7.1 Run the full crate test suite repeatedly (at least ten consecutive runs) to confirm the
      crash tests are not timing-sensitive, and record the added wall-clock cost of the suite.
- [x] 7.2 Verify clippy, `cargo test`, and treefmt are green across the workspace, and that the added
      binary target has not changed anything downstream of the bedrock crate.
