## Context

See proposal.md — Why. What the tree holds on 2026-10-14:

- **Logging today.** `crates/outpost` and `crates/outpost-fabric` contain 21 `tracing` calls: 16 `warn!`, 3 `info!`, 1 `debug!`, 1 `error!`. All are formatted strings (`"station: registration failed: {e}"`), none carry fields. The Station's successful paths — `register` bootstrapping a hold, `dispatch` admitting a session, `subscribe_session` parking and reaping, the orderer countersigning — log nothing. The binary logs `station running` (with fields), the termination signal, and the close timeout.
- **What the Station knows at each point.** `register` has the `TerritoryID`, the folded ACL, the `Caller` (identity and device cert) when authenticated, and the policy outcome as a `bool`. `dispatch` has the request kind, the caller, and the replica lookup. `subscribe_session` ends its live loop on one of three breaks — idle, lagged, closed by the peer — currently indistinguishable. `Replica::order` returns `Option<OrderProof>`, folding "does not chain" and "no replica" into `None` at the call site.
- **Ids.** `TerritoryID::as_bytes()` and `hex::encode` are how holds are named on disk; `MemberID` and device keys are 32 bytes; the audit harness and the hold file names already use lowercase hex, so the operator can correlate.
- **The filter.** `main.rs` builds the subscriber from `EnvFilter::try_from_default_env()` falling back to `outpost=info`, before the runtime starts; there is no flag.
- **Tests that read the log.** `tests/common/process.rs` already spawns the binary with `--log-format json` and parses each line as JSON to find `station running`; the event tests extend that reader.
- **Content-blindness.** The Station's module docs state it holds no read keys and contains no decryption code. Events must not weaken that in a new direction: the log is a side channel that a collector retains.

## Goals / Non-Goals

**Goals:**

- Every operational outcome a Station decides is visible as an event with the fields an operator needs to act, and the set of events is a contract.
- The log can be proven payload-free by a test, not by review.

**Non-Goals:**

- Beyond the proposal's: no new log sink, no sampling or rate limiting (the noisiest event, `subscription parked`, is at debug), no event for every frame.

## Decisions

### D1 — Events are `tracing` calls at the decision site, named by the message

**Chosen** (2026-10-14). Each event is a `tracing::info!`/`warn!`/`debug!` with the event name as the message and the fields as structured fields (`territory = %hex, registrant = %hex`), placed where the decision is made: in `register` for the registration events, in `dispatch` for session refusals, in `subscribe_session` for park and reap, at the `OrderRequest` arm for ordering. A refusal's reason is a `&'static str` from the closed set; a small private enum with `as_str()` keeps the set closed at compile time.

**Why not a logging layer or an event type.** The decisions are already localised in the Station; an `Event` enum emitted from a central place would mean threading outcomes back up for the sake of one call. `tracing` is the framework the crate and the app use; its JSON renderer gives the format the spec requires without a serializer of our own.

**Rejected.** _An `events` module with constructor functions per event_ — keeps the names in one file, but every call site then imports a function that wraps one macro call; the spec table is the single list, and the test in D4 enforces it.

### D2 — Reasons are made explicit where they are currently folded

`register` returns `Ok(false)` for four distinct reasons; it gains a private `Refusal` enum and returns `Result<Registered, Refusal>`, with the public `bool` ack derived at the call site. `subscribe_session`'s three `break`s set a `reason` before breaking. The `OrderRequest` arm distinguishes "no replica" and "no fleet key" (both known at the arm) from `Replica::order` returning `None` ("does not chain"). No wire message changes: the Vessel still sees `Ack { ok: false }` and `OrderResponse { proof: None }`, because telling a stranger why they were refused is not this change.

### D3 — The scan and shutdown events live in the binary

`holds loaded` is emitted by `commands/station.rs` around the `Station::hosting_*` call, with the count read from the Station after construction and the duration measured there; the library's `scan_holds` stays silent so embedders (tests, the app) are not told about a scan they did not ask about. `shutting down` carries the number of live sessions, which the Station exposes through a new `live_sessions()` count maintained by the serve loop (increment on accept, decrement on handle return). `station stopped` carries the close duration and whether the 10 s bound was hit — the existing timeout branch, with fields.

### D4 — The spec table is the test's table

`tests/events.rs` holds the event table as data (name, level, required fields) and drives the binary through each path that can be reached from outside — register, re-register, refused register, sync as a non-member, subscribe and idle out with a short liveness, order, SIGTERM — asserting each expected event appears with its fields and level in the JSON log. A separate case authors a Notice with a sentinel plaintext, syncs it with the floor at `RUST_LOG=outpost=trace`, and asserts the sentinel, the ciphertext, and the envelope bytes appear nowhere in the log. Events reachable only through library embedding (`subscription parked` under a long liveness) are covered by the same table against an in-process Station with a `tracing` test subscriber.

### D5 — One floor from `RUST_LOG`; levels are assigned so the floor means something

**Chosen** (2026-10-14, after review). The `EnvFilter` built from `RUST_LOG` with the `outpost=info` fallback stays the single control, documented in `outpost station`'s long help and as a commented `env` entry in the StatefulSet. A `--log-filter` option was drafted and dropped: it added no capability over `RUST_LOG`, it needed a precedence rule, and the operator's model is a floor the Station emits at, with any narrowing done downstream by the collector, a UI, or `grep`.

The consequence lands on the table, not on the binary: a level is a promise about volume and importance. `info` must stay low-volume — once per registration, per start, per reap, per stop — because the default floor is meant to be kept on in production and read whole. `debug` holds per-session chatter (`subscription parked`, `territory re-registered`, `order granted`). An event that could fire per frame belongs at `trace`, and none in the table does. Adding an event means choosing its level against that rule.

## Risks / Trade-offs

- [Events name members and devices by public key, and a collector retains them] → Stated in the spec and the proposal; the hosted tier's retention policy is the control. Nothing here is new information to the Station, but it is new information to whoever holds its logs.
- [`info` events on every registration could be noisy on a Fleet with many first-time registrations] → Registrations are rare relative to sessions; the per-session events are at debug or are refusals. If a hosted Fleet proves otherwise, raising the floor to warn is the knob, and the event's level is the thing to revisit.
- [Threading reasons through `register` changes a private signature used by tests] → Private; the tests call over the wire.
- [The field-name contract is enforced by a test, not by types] → A renamed field fails the table test; a new event not in the table does not fail anything, which is acceptable: the contract is that named events keep their shape, not that no others exist.

## Migration Plan

1. Land the library events and the reason enums with the in-process coverage.
2. Land the binary events, the documented floor, and the process-level table test.
3. Update the architecture reference.
4. Rollback is per step; no wire or on-disk format changes.

## Open Questions

- Whether `territory registered` should also be written to the Station's own database as an operator-queryable registration log. The row already exists (`record_registration` stores who and when); the question is only whether a CLI reads it, and that is a tooling question for the roster change.
