## Context

See proposal.md — Why. What the tree holds on 2026-10-16:

- **The Station's database.** `crates/outpost-hold/src/station_hold.rs`: `station.db`, a bedrock database beside the Territory holds, with two collections declared through the `Collection` derive — `SightingRow` (a 32-byte synthetic id, an indexed `territory`, cert bytes, `first_seen`/`last_seen` as `i64`) and `RegistrationRow` (the Territory id as key, `registered_by`, `registered_at`). `StationHold::open` runs `ensure_collection` for each, which creates or additively migrates. `Station::register` writes a registration row on a first authenticated registration and warns, rather than fails, when the write does not go through.
- **bedrock's surface for a log-shaped table.** `Database::{insert, get, query, all, count, delete, update_where}`; `Query` takes a `Filter` tree (`eq/ne/gt/gte/lt/lte`, `in`, `exists`, JSON-path compare), sort keys, and a limit; integer scalars are `i64` columns and can be ids and indexes. There is no delete-by-filter: `update_where` exists, its delete sibling does not. Documents are `jsonb`; a free-form JSON field is either a `String` column or a document type the derive accepts, which the first task settles.
- **Logging today.** 21 `tracing` calls across `outpost` and `outpost-fabric`, 16 of them `warn!`, all formatted strings without fields. The Station's successful paths log nothing. `main.rs` installs an `EnvFilter` from `RUST_LOG` with `outpost=info` as the fallback, before the runtime starts; `--log-format` already selects the JSON renderer. `tests/common/process.rs` spawns the binary and parses its JSON log line by line.
- **What the Station knows at each decision.** `register` has the `TerritoryID`, the folded ACL, the `Caller` when authenticated, and four distinct reasons to say no that all become `Ok(false)`. `dispatch` has the request kind and the replica lookup. `subscribe_session`'s live loop ends on idle, lagged, or closed, indistinguishable today. `Replica::order` returns `Option<OrderProof>`, folding "does not chain" with the arm-level "no replica" and "no fleet key".
- **Ids.** Holds are named by `hex::encode(territory.as_bytes())`; `MemberID` and device keys are 32 bytes; the audit harness already correlates by lowercase hex.
- **The term.** The published specs say "operator" for the person who runs a Fleet. Kubernetes uses the same word for a controller pattern, and the Territory ACL has an `Admin` permission, so neither "operator" nor "administrator" is free. The person is the Fleet's commodore from here on; the definition lands in `sync-protocol`'s Purpose, which is still the archive placeholder.

## Goals / Non-Goals

**Goals:**

- Every decision a Station makes is a durable row before it is a log line, and a log line is provably a row.
- The set of events is a contract; diagnostics are free to change.
- The table cannot fill the volume: retention is a duration, and pruning is automatic.
- The log can be proven payload-free by a test, not by review.

**Non-Goals:**

- Beyond the proposal's: no reader, no sampling, no event for every frame, no replication of the table.

## Decisions

### D1 — An event is a row first; the line is produced from the row

**Chosen** (2026-10-16). `StationHold` gains `EventRow { id: EventSeq(i64), at: i64, kind: String, fields: String }` — `at` indexed, `fields` a JSON object as text (a document column if the derive takes a free-form value; same contract either way). `append_event` inserts with the next sequence, which the hold reads as the highest id at open and then increments in process. The Station's `record` path builds the field map, appends, and only then emits the `tracing` record with the same fields plus `seq`. A failed append emits `events table write failed` as a diagnostic warning and no event.

The row and the line cannot drift because one private macro produces both from a single field list: `event!(self, INFO, "territory registered", territory = hex, registrant = hex)` expands to the JSON map for the row and the `tracing::info!` call for the line. `tracing` cannot take a dynamic field set, so the field list has to be static at the call site anyway; making the macro the only way to emit an event is what turns "produced from the row" into a property rather than a discipline.

**Why a table and not just the log.** A log stream has no delivery guarantee and no identity; a row has both, and the sequence number gives the stream one by reference. The Station already keeps durable per-Station facts (sightings, registrations) in exactly this database.

**Rejected.** _Writing the line first and the row after_ — the line could then claim what never committed. _A generic emitter that serializes the map into one `fields` string_ — loses the per-field filtering a collector wants from JSON logs.

### D2 — Events are decisions; everything else is a diagnostic

An event records something the Station decided or completed: homed, refused, reaped, ordered-and-refused, loaded, running, stopping, stopped. A diagnostic records a failure or a per-session detail. The split decides what goes in the table and what the contract covers: the table in the spec lists events and their levels; diagnostics keep today's `warn!`/`debug!` calls, gaining fields. Three things that were events in the first draft moved to diagnostics for a reason each: a repeat registration decides nothing new; a parked subscription is per-session and would be the noisiest row; an order granted is already recorded durably and authoritatively in the Territory's ACL log at that position, so a second record would be weaker than the first.

### D3 — Reasons are made explicit where they are folded

`register` gains a private `Refusal` enum (`Policy`, `InvalidLog`, `MismatchedId`, `NoStorage`) and returns `Result<Registered { new: bool }, Refusal>`, with the wire `Ack { ok }` derived at the call site. `subscribe_session`'s three loop exits set a `Reap` reason before breaking. The `OrderRequest` arm separates "no replica" and "no fleet key" (known at the arm) from `Replica::order` returning `None` ("does not chain"). No wire message changes: a stranger still sees `Ack { ok: false }` and `OrderResponse { proof: None }`; telling callers why they were refused is not this change.

### D4 — Retention is a duration, pruned on the write path, and the newest row is immortal

`--events-retain-days` (`OUTPOST_EVENTS_RETAIN_DAYS`), default 90, parsed as `u32` with 0 refused at start-up. The hold prunes `at < now - retention` at open and after every append, with `delete_where::<EventRow>`, a new bedrock façade method mirroring `update_where` (a `DELETE … WHERE` from the same filter rendering; a few lines plus a test against the real engine). The prune excludes the highest id, so the newest event is kept whatever its age: that row is what makes the sequence recoverable after an idle stretch longer than the retention, without a separate counter table.

**Why on append rather than a timer.** Events arrive at the rate of decisions, which is low, and an indexed delete of nothing is cheap; a timer is a second task with its own shutdown ordering for no benefit. **Why days.** It is the granularity a commodore thinks in; a duration parser would be a dependency for a knob that will be set once.

### D5 — One floor from `RUST_LOG`; levels are assigned so the floor means something

The `EnvFilter` from `RUST_LOG` with the `outpost=info` fallback stays the single control, named in `outpost station`'s long help and as a commented `env` entry in the StatefulSet. A `--log-filter` option was drafted and dropped: it added nothing over `RUST_LOG`, needed a precedence rule, and the commodore's model is a floor the Station emits at, with any narrowing done downstream by the collector, a UI, or `grep`. The table is not subject to the floor at all; D1's append happens before the filter is consulted.

The consequence lands on the table of kinds: every event is info or warn, so the default stream carries all of them, and `info` stays low-volume by construction — once per registration, refusal, reap, start, stop. Diagnostics live at debug (per-session detail) or warn (failures). Anything that could fire per frame would be `trace`, and nothing does.

### D6 — The read path is a hold method, and nothing reads it yet

`StationHold::events_since(seq, limit) -> Vec<EventRow>`: `Query` filtered on `id > seq`, sorted ascending, limited. It is the foundation for a reader — a CLI, an endpoint on the probe listener, a UI — and this change ships none, because none is on the path to a real deployment. It is cheap to add now (one query) and it keeps the first reader from reaching into the table's shape.

### D7 — The scan and shutdown events live in the binary

`holds loaded` is recorded by `commands/station.rs` after `Station::hosting_*` returns, with the count read from the Station and the duration measured around the constructor; the library's `scan_holds` stays silent so embedders are not told about a scan they did not ask about. `shutting down` carries `live_sessions()`, a count the serve loop maintains. `station stopped` carries the close duration and whether the 10 s bound was hit — the existing timeout branch, as an event. All three go through the same record path; the hold is open by then.

### D8 — Refusals are coalesced per source and reason; admission rate-limiting is the real fix

A refused session or registration costs the Station a row, and refusals are attacker-paced: a stranger can connect, be refused, and repeat as fast as it can complete QUIC handshakes. The Station keeps a small in-memory map keyed by (remote address, reason) holding a count, the time of the first refusal, and the identity and Territory when known. On a refusal with no live entry, the event is recorded at once with `count` 1 and the entry opened; on one within the window (60 s), the entry is incremented and nothing is written. Entries whose window has passed are flushed — one event carrying `count` and `since` — whenever the Station records any event, and at `shutting down`; there is no timer, so an idle Station after a flood flushes the tail at its next event, which is at the latest the shutdown. The key is the connection's remote address rather than its transport key, because keys are free to mint per connection and addresses are not; the address is the coalescing key only and is never written to the row, so the metadata-only rule stays as narrow as it was.

**What this bounds, and what it does not.** It bounds the record: a flood produces one row per source per minute, and the log stays readable during one. It does not bound the flood: the Station still completes every handshake and runs admission for each refusal. The long-term fix is admission rate-limiting — refusing or delaying connections from an address that has been refused recently, before the handshake is paid for — which belongs with the admission-policy change the hosted tier needs (quotas, further `RegisterPolicy` arms), where what a stranger may cost a Station is decided once. Named here so that coalescing is not mistaken for that fix.

## Risks / Trade-offs

- [A refused session is one row, and refused sessions are attacker-paced] → Coalesced per source and reason (D8), so the record grows per source per minute rather than per attempt. The handshake cost is untouched; admission rate-limiting, later, is what bounds it.
- [Coalesced counts live in memory until flushed] → A crash loses at most one window's tail per source; the first refusal of each window is already on disk. Shutdown flushes.
- [Every event is a write on the single serialized writer] → Events happen at decision rate; the hold's existing write-through for sightings is on the same path at a higher rate.
- [Events name members and devices by public key, now durably on the volume as well as in the stream] → Stated in the spec and proposal; the hosted tier's retention policy covers both the table (by the retention setting) and the collector (by its own).
- [A storage-less Station records nothing, so in-process tests that use `Station::new` see no events] → The event tests use `hosting` and the binary; `new` is for single-Territory embeds.
- [The `fields` column's shape is settled in the first task] → Either outcome keeps the same contract: a JSON object per row.
- [`delete_where` is a bedrock change inside an outpost change] → A façade addition with no spec-level behavior; tested against the real engine, kept to the shape of `update_where`.

## Migration Plan

1. Land `delete_where` and the hold's events table with engine tests.
2. Land the Station's record path, the reason enums, and the diagnostics with fields.
3. Land the binary: retention option, scan and shutdown events, the floor in the help.
4. Land the event, sentinel, retention, and floor tests.
5. Update the architecture reference and the manifests' comments.
6. Rollback per step: an older binary ignores an events table it never reads, and the collection is additive.
