## Why

The storage layer was rebuilt on `bedrock` (typed documents over turso) and SeaORM was removed
entirely. Whatever durability verification existed before that rebuild no longer applies to the
code that actually persists user data today. The capability survived the migration — the pool sets
`journal_mode = WAL` and a `durability` module exposes checkpoint/flush primitives — but nothing
proves the posture we believe we have, and nothing stops it from regressing.

Investigation of the current code turned up three concrete holes:

1. **The `synchronous` level is never set by Waypoints.** `Config::default()` ships an empty pragma
   map, so crash safety rests entirely on turso's per-connection default of `SyncMode::Full`. That
   default is a _deliberate upstream deviation_ from SQLite (which defaults to `NORMAL` under WAL) —
   turso's own source comments say so. Depending on an intentional upstream deviation, unasserted,
   is the single largest durability risk in the layer: an engine bump could silently downgrade every
   territory to `NORMAL` with no test failing.
2. **Pragma application is best-effort and unverified.** `apply_pragma` logs a rejection to stderr
   and continues, so a `journal_mode = WAL` that the engine refuses produces a working database in
   an unknown journal mode. No test reads `PRAGMA journal_mode` back.
3. **The documented close-time behaviour is false.** `Database::close()` drops connections; it never
   calls turso's `Connection::close()`, and turso's `Drop` impl deliberately skips the shutdown
   checkpoint that `close()` performs. A comment in `store` asserts "the bedrock engine flushes WAL
   on close" — it does not. This is not data loss (committed frames are already fsynced and replayed
   on reopen), but it is a belief the code does not support.

This change is therefore mostly **verification, not implementation**: state the durability
invariants as testable facts, prove them against the real engine, and guard them.

## What Changes

- **Establish the durability invariants as spec statements**: journal mode, `synchronous` level,
  checkpoint behaviour (automatic and explicit), what `close()` does and does not do, and — stated
  precisely — which failure classes the guarantees cover.
- **Set `synchronous` explicitly.** `Config::default()` gains an explicit `synchronous = FULL`
  applied to every connection, replacing reliance on the engine default. Callers can still lower it
  through the existing pragma map; nothing in Waypoints does.
- **Make the WAL pragma verifiable.** After the pool opens the write connection, read
  `PRAGMA journal_mode` and `PRAGMA synchronous` back and report a structured mismatch rather than
  leaving a stderr line as the only evidence. The open still succeeds — a durability mismatch must
  not deny the user access to notes that are on disk and readable. Instead the mismatch is surfaced
  to the user with available remediation and a pre-filled bug report. Caller-supplied pragmas keep
  their best-effort, diagnostic-only contract.
- **Gate durability before release.** The pragma assertions run against the real engine as a release
  precondition, and are re-run when accepting any storage-engine upgrade. The engine is pre-v1 and
  deliberately pinned, so an upstream default change must fail a test rather than drift silently
  into production. This — not the runtime check — is the primary control.
- **Crash-recovery test suite against the real engine.** A dedicated victim binary writes to a real
  file-backed territory and is `SIGKILL`ed by the test at a chosen point; the test then reopens the
  database and asserts integrity and content. Covered: in-flight single insert, in-flight explicit
  transaction (`insert_all`, which runs `BEGIN IMMEDIATE`), and reopen-after-kill with a populated
  territory.
- **Regression guard.** A test asserts the effective journal mode and synchronous level read back
  from a live connection, so weakening either fails CI instead of shipping.
- **Correct the two false comments** (`durability.rs`'s "fsync the page cache" description of
  `cacheflush`, and `store`'s "flushes WAL on close"), so the code no longer documents a guarantee it
  does not provide.

Honest limit, stated up front and repeated in the spec: **true power-loss durability is not testable
in CI.** Killing a process proves the engine's own recovery path; it does not prove the platform
honoured an fsync, and no test in this repository will claim otherwise. The tests establish
process-kill and abrupt-close recovery. Power-loss and OS-crash behaviour is asserted as a
configuration property (`synchronous = FULL` under WAL), not as a verified outcome.

## Capabilities

### New Capabilities

- `storage`: seeds the `storage` domain spec with its durability section — journal mode,
  synchronous level, checkpoint and flush semantics, close semantics, and the recovery guarantees
  that hold after abnormal termination. The domain's other concerns (collection specs, migrations,
  query semantics) are untouched by this change and remain unspecified until a change covers them.

### Modified Capabilities

<!-- None. `openspec/specs/` is empty; this change seeds the storage domain rather than amending it. -->

## Impact

- **`crates/bedrock/src/config.rs`** — `Config::default()` gains an explicit synchronous setting and
  the field/doc to describe it.
- **`crates/bedrock/src/pool.rs`** — durability pragmas are read back and verified after the write
  connection opens; `apply_pragma`'s best-effort contract is narrowed to caller-supplied pragmas.
- **`crates/bedrock/src/durability.rs`** — doc correction for `FlushMode::Fsync`; no behaviour change.
- **`crates/bedrock/src/error.rs`** — one new error variant for a durability pragma that did not
  take effect.
- **`crates/bedrock/tests/`** — new crash-recovery and durability-configuration integration tests.
- **`crates/bedrock/Cargo.toml`** — a dev-only `[[bin]]` crash-victim target the tests spawn and kill.
- **`crates/store/src/tools.rs`** — comment correction only.
- No dependency additions. No schema, migration, or collection-spec changes. No public API removal;
  the `Config` addition is additive and defaults to the level the engine already applied.

## Non-goals

- **No backup or export of territory databases.** Snapshotting, exporting, or restoring a territory
  is a separate concern and separately tracked. This change does not make a corrupted database
  recoverable — it establishes that abnormal termination does not corrupt one.
- **No sync, replication, or multi-device durability.** Nothing here touches the sync path; a
  territory's durability is a local, single-process property in this change.
- **No performance tuning or benchmarking.** `synchronous = FULL` is chosen for crash safety, not
  measured against alternatives. No throughput work beyond keeping the crash tests fast enough to
  run per-commit.
- **No schema or migration changes.** No collection spec, DDL, or `user_version` behaviour is
  modified.
- **No claim of verified power-loss durability.** The tests do not simulate power loss, torn writes,
  or a lying `fsync`, and the spec does not assert an outcome they cannot establish.
