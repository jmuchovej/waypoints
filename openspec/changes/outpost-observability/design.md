## Context

See proposal.md — Why. What the tree holds on 2026-10-16:

- **The Station's database.** `crates/outpost-hold/src/station_hold.rs`: `station.db`, a bedrock database beside the Territory holds, with two collections declared through the `Collection` derive — `SightingRow` (a 32-byte synthetic id, an indexed `territory`, cert bytes, `first_seen`/`last_seen` as `i64`) and `RegistrationRow` (the Territory id as key, `registered_by`, `registered_at`). `StationHold::open` runs `ensure_collection` for each, which creates or additively migrates. `Station::register` writes a registration row on a first authenticated registration and warns, rather than fails, when the write does not go through.
- **bedrock's columns.** `bedrock-core` implements `Field` for `bool`, `String`, `[u8; N]`, the integers, and `Option<T>` (a nullable column), so a row whose fields are known at compile time is a struct of typed columns with no document. `Database::{insert, get, query, all, count, delete, update_where}`; `Query` takes a `Filter` tree (`eq/ne/gt/gte/lt/lte`, `in`, `exists`), sort keys, and a limit. There is no delete-by-filter: `update_where` exists, its delete sibling does not. Documents (`jsonb`) exist for structures unknown until runtime; none of this change's structures is.
- **Durations.** `jiff` 0.2 is already in the lockfile (through `serde_with`). Its "friendly" duration grammar accepts `y`/`yr`/`years`, `mo`/`mos`/`months`, `w`/`wk`/`weeks`, `d`/`days`, `h`, `m`, `s`, with or without spaces, and the ISO 8601 form; `Span` implements `FromStr`, so clap takes it as a typed value. Months and years are calendar units: a cutoff is computed by subtracting the span from a zoned "now", not by multiplying a fixed month length.
- **Logging today.** 21 `tracing` calls across `outpost` and `outpost-fabric`, 16 of them `warn!`, all formatted strings without fields. The Station's successful paths log nothing. `main.rs` installs an `EnvFilter` from `RUST_LOG` with `outpost=info` as the fallback, before the runtime starts; `--log-format` already selects the JSON renderer. `tests/common/process.rs` spawns the binary and parses its JSON log line by line.
- **What the Station knows at each decision.** `register` has the `TerritoryID`, the folded ACL, the `Caller` when authenticated, and four distinct reasons to say no that all become `Ok(false)`. `dispatch` has the request kind and the replica lookup. `subscribe_session`'s live loop ends on idle, lagged, or closed, indistinguishable today. `Replica::order` returns `Option<OrderProof>`, folding "does not chain" with the arm-level "no replica" and "no fleet key".
- **Ids.** Holds are named by `hex::encode(territory.as_bytes())`; `MemberID` and device keys are 32 bytes; the audit harness already correlates by lowercase hex.
- **Names.** The person who runs a Fleet is the commodore, defined in `sync-protocol`'s Purpose on the implementation branch. The `station` subcommand's five options read `OUTPOST_STATION_{DATA,KEY_FILE,FLEET_KEY_FILE,PORT,ADVERTISE}` after the rename on that branch; the global `--log-format` reads `OUTPOST_LOG_FORMAT`; the audit harness's `OUTPOST_AUDIT_*` are test knobs, not Station options.

## Goals / Non-Goals

**Goals:**

- Every decision a Station makes is a durable row before it is a log line, and a log line is provably a row.
- The set of events is a contract; diagnostics are free to change.
- The table's shape is the kinds' shape: typed columns, checked by the compiler, no document.
- The table cannot fill the volume: retention is a period, and pruning is scheduled.
- The log can be proven payload-free by a test, not by review.

**Non-Goals:**

- Beyond the proposal's: no reader, no sampling, no event for every frame, no replication of the table.

## Decisions

### D1 — An event is a typed row first; the line is produced from the row

**Chosen** (2026-10-16). `StationHold` gains `EventRow`, one struct whose columns are the union of the spec's fields:

| Column                                             | Type                          | Used by                                                  |
| -------------------------------------------------- | ----------------------------- | -------------------------------------------------------- |
| `id`                                               | `EventSeq(i64)`, the sequence | every kind                                               |
| `at`                                               | `i64`, indexed                | every kind                                               |
| `kind`                                             | `String`, indexed             | every kind                                               |
| `territory`                                        | `Option<[u8; 32]>`, indexed   | registered, both refusals, reaped, order refused         |
| `member`, `device`                                 | `Option<[u8; 32]>`            | registered, both refusals, reaped, order refused; reaped |
| `reason`                                           | `Option<String>`              | both refusals, reaped, order refused                     |
| `count`, `since`                                   | `Option<i64>`                 | coalesced refusals; `count` also for holds loaded        |
| `duration_ms`, `close_ms`, `open_sessions`, `port` | `Option<i64>`                 | holds loaded, stopped, shutting down, running            |
| `orderer`, `timed_out`                             | `Option<bool>`                | running, stopped                                         |
| `data`, `address`                                  | `Option<String>`              | running                                                  |

`append_event` inserts with the next sequence, which the hold reads as the highest id at open and then increments in process. The Station's `record` path fills the row, appends, and only then emits the `tracing` record with the same fields plus `seq`. A failed append emits `events table write failed` as a diagnostic warning and no event.

The row and the line cannot drift because one private macro produces both from a single field list: `event!(self, INFO, "territory registered", territory, member)` expands to a struct literal over `EventRow::default()` and to the `tracing::info!` call with the same names. The field names are struct fields, so a misspelled one is a compile error; the previous draft's JSON map would have made it a typo in a key.

**Why one table with nullable columns, not a table per kind.** The sequence is global across kinds, and `events_since` is one query; per-kind tables would need a shared sequence table and a nine-way union to read. Most columns are null on most rows, which costs a byte per null in SQLite's record format on a table that retention keeps small. **Why `member` for every "who".** Four kinds each had their own name for the same 32-byte member id (registrant, identity, author, member); one column with one name maps one-to-one onto the line, and the kind already says which role the member played. **Why `address` is text.** It is the pasted address a commodore copies into the app, a value nobody filters on; its queryable part, `port`, has its own column, and the Station's identity is constant. It is the only text column that happens to hold JSON, and the design says so rather than pretending otherwise.

**Rejected.** _Writing the line first and the row after_ — the line could then claim what never committed. _A JSON document column for the per-kind fields_ (the first draft) — documents are for structures unknown at compile time; these are closed, and the engine is built for columns. _A generic emitter serializing a map_ — loses both the compiler's check and per-field filtering in the collector.

### D2 — Events are decisions; everything else is a diagnostic

An event records something the Station decided or completed: homed, refused, reaped, ordered-and-refused, loaded, running, stopping, stopped. A diagnostic records a failure or a per-session detail. The split decides what goes in the table and what the contract covers: the table in the spec lists events and their levels; diagnostics keep today's `warn!`/`debug!` calls, gaining fields. Three things that were events in the first draft moved to diagnostics for a reason each: a repeat registration decides nothing new; a parked subscription is per-session and would be the noisiest row; an order granted is already recorded durably and authoritatively in the Territory's ACL log at that position, so a second record would be weaker than the first.

### D3 — Reasons are made explicit where they are folded

`register` gains a private `Refusal` enum (`Policy`, `InvalidLog`, `MismatchedId`, `NoStorage`) and returns `Result<Registered { new: bool }, Refusal>`, with the wire `Ack { ok }` derived at the call site. `subscribe_session`'s three loop exits set a `Reap` reason before breaking. The `OrderRequest` arm separates "no replica" and "no fleet key" (known at the arm) from `Replica::order` returning `None` ("does not chain"). No wire message changes: a stranger still sees `Ack { ok: false }` and `OrderResponse { proof: None }`; telling callers why they were refused is not this change.

### D4 — Retention is a period in jiff's grammar, pruned by a scheduled task

`--retention-period` (`OUTPOST_STATION_RETENTION_PERIOD`) is a `jiff::Span`, default `3mo`. At start-up, and at each prune, the Station computes the cutoff as the zoned current time minus the span, and refuses to start if the span is shorter than one day, compared the same way. `jiff` joins the `cli` feature; it is already in the build graph, and no parser is written here.

**Calendar units are calendar-accurate, not averaged.** jiff subtracts a month as a month and clamps into a shorter one — its own doc examples give `2019-01-31 + 1.months() = 2019-02-28` and `2024-03-31 - 1.months() = 2024-02-29`. So `3mo` retains back to the same day of the month three months earlier: between 89 and 92 days depending on when the prune runs, shortest across February. It is _not_ the 30.44-day average month some duration parsers use (humantime defines one that way), and nothing here converts the span to a fixed number of seconds — which jiff would refuse without a reference date anyway. Keeping `3mo` as the default is deliberate: "three months" is what a commodore means, and a day and a half of drift on a ninety-day window changes nothing about it. A commodore who wants a window of exactly known length writes days (`90d`), which the spec states and the option's help repeats.

Pruning is a task spawned beside the serve loop: it prunes once at start-up, then on an hourly tick, with `delete_where::<EventRow>` on `at < cutoff` excluding the highest id — a new bedrock façade method mirroring `update_where` (a `DELETE … WHERE` from the same filter rendering; a few lines plus a test against the real engine). The exclusion keeps the newest event whatever its age, which is what makes the sequence recoverable after an idle stretch longer than the retention, without a separate counter table. The task is aborted at shutdown; a prune is one indexed delete in one transaction, so an abort mid-way loses nothing. The tick interval is a parameter of the task for tests and is not exposed as an option.

**Why hourly.** Retention floors at a day, so an hour is at most a twenty-fourth of the shortest retention, and an indexed delete that finds nothing is free. **Why not on every append** (the first draft). A delete per event is work done when nothing is old, on the hot path, for no benefit a scheduled task does not give; decoupling it also keeps the record path a single insert. **Why a duration string.** `90` is a number a commodore has to look up the unit of; `3mo` is what they meant. The grammar is jiff's, so what it accepts is documented upstream rather than by us.

### D5 — One floor from `RUST_LOG`; levels are assigned so the floor means something

The `EnvFilter` from `RUST_LOG` with the `outpost=info` fallback stays the single control, named in `outpost station`'s long help and as a commented `env` entry in the StatefulSet. A `--log-filter` option was drafted and dropped: it added nothing over `RUST_LOG`, needed a precedence rule, and the commodore's model is a floor the Station emits at, with any narrowing done downstream by the collector, a UI, or `grep`. The table is not subject to the floor at all; D1's append happens before the filter is consulted.

The consequence lands on the table of kinds: every event is info or warn, so the default stream carries all of them, and `info` stays low-volume by construction — once per registration, refusal, reap, start, stop. Diagnostics live at debug (per-session detail) or warn (failures). Anything that could fire per frame would be `trace`, and nothing does.

### D6 — The read path is a hold method, and nothing reads it yet

`StationHold::events_since(seq, limit) -> Vec<EventRow>`: `Query` filtered on `id > seq`, sorted ascending, limited. It is the foundation for a reader — a CLI, an endpoint on the probe listener, a UI — and this change ships none, because none is on the path to a real deployment. It is cheap to add now (one query) and it keeps the first reader from reaching into the table's shape.

### D7 — The scan and shutdown events live in the binary

`holds loaded` is recorded by `commands/station.rs` after `Station::hosting_*` returns, with the count read from the Station and the duration measured around the constructor; the library's `scan_holds` stays silent so embedders are not told about a scan they did not ask about. `shutting down` carries `live_sessions()`, a count the serve loop maintains. `station stopped` carries the close duration and whether the 10 s bound was hit — the existing timeout branch, as an event. All three go through the same record path; the hold is open by then.

### D8 — Refusals are coalesced per source and reason; admission rate-limiting is the real fix

A refused session or registration costs the Station a row, and refusals are attacker-paced: a stranger can connect, be refused, and repeat as fast as it can complete QUIC handshakes. The Station keeps a small in-memory map keyed by (remote address, reason) holding a count, the time of the first refusal, and the member and Territory when known. On a refusal with no live entry, the event is recorded at once with `count` 1 and the entry opened; on one within the window (60 s), the entry is incremented and nothing is written. Entries whose window has passed are flushed — one event carrying `count` and `since` — whenever the Station records any event, and at `shutting down`; there is no timer, so an idle Station after a flood flushes the tail at its next event, which is at the latest the shutdown. The key is the connection's remote address rather than its transport key, because keys are free to mint per connection and addresses are not; the address is the coalescing key only and is never written to the row, so the metadata-only rule stays as narrow as it was.

**What this bounds, and what it does not.** It bounds the record: a flood produces one row per source per minute, and the log stays readable during one. It does not bound the flood: the Station still completes every handshake and runs admission for each refusal. The long-term fix is admission rate-limiting — refusing or delaying connections from an address that has been refused recently, before the handshake is paid for — which belongs with the admission-policy change the hosted tier needs (quotas, further `RegisterPolicy` arms), where what a stranger may cost a Station is decided once. Named here so that coalescing is not mistaken for that fix.

### D9 — Environment names: `OUTPOST_STATION_*` for the subcommand, `OUTPOST_*` for the binary

A `station` option's variable is `OUTPOST_STATION_<OPTION>`; a global option's is `OUTPOST_<OPTION>`. The binary is `outpost`, so an unprefixed `STATION_*` would be one more name to collide with in a pod's environment, and the subcommand segment says which process the setting belongs to once `fleet` and `station id` exist beside `station`. The five existing station options were renamed in one commit on `worktree/deployable-station` (the flags, the test harness's scrub list, the manifests, the image's environment); the archived `deployable-station` change keeps the old names as the record of what it decided at the time.

## Risks / Trade-offs

- [A refused session is one row, and refused sessions are attacker-paced] → Coalesced per source and reason (D8), so the record grows per source per minute rather than per attempt. The handshake cost is untouched; admission rate-limiting, later, is what bounds it.
- [Coalesced counts live in memory until flushed] → A crash loses at most one window's tail per source; the first refusal of each window is already on disk. Shutdown flushes.
- [Pruning lags the period by up to an hour] → Invisible against a one-day floor; the task's interval is a parameter so tests do not wait.
- [Most columns are null on most rows] → A byte per null in the record; the table is bounded by retention. A per-kind layout would cost a shared sequence and a union read, which is the worse trade.
- [Every event is a write on the single serialized writer] → Events happen at decision rate; the hold's existing write-through for sightings is on the same path at a higher rate.
- [Events name members and devices by public key, now durably on the volume as well as in the stream] → Stated in the spec and proposal; the hosted tier's retention policy covers both the table (by the retention period) and the collector (by its own).
- [A storage-less Station records nothing, so in-process tests that use `Station::new` see no events] → The event tests use `hosting` and the binary; `new` is for single-Territory embeds.
- [`delete_where` is a bedrock change inside an outpost change] → A façade addition with no spec-level behavior; tested against the real engine, kept to the shape of `update_where`.
- [`jiff` becomes a direct dependency] → Already built for `serde_with`; the `cli` feature only names it. Its grammar, not ours, defines what a period string may be.

## Migration Plan

1. Land `delete_where` and the hold's typed events table with engine tests.
2. Land the Station's record path, the reason enums, coalescing, and the diagnostics with fields.
3. Land the binary: the retention period, the prune task, the scan and shutdown events, the floor in the help.
4. Land the event, sentinel, retention, floor, and coalescing tests.
5. Update the architecture reference and the manifests' comments.
6. Rollback per step: an older binary ignores an events table it never reads, and the collection is additive.
