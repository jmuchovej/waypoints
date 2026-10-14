## 1. Library events

- [ ] 1.1 Give `register` in `crates/outpost/src/station.rs` a private `Refusal` enum (`Policy`, `InvalidLog`, `MismatchedId`, `NoStorage`) with `as_str()` in the spec's wording, return `Result<Registered { new: bool }, Refusal>`, and emit `territory registered` (info), `territory re-registered` (debug), or `registration refused` (warn) with `territory`, `registrant`, and `reason` as hex and static strings; verify `cargo test -p outpost --test hosting --test admission --test fleet` still passes and the ack a Vessel sees is unchanged
- [ ] 1.2 In `dispatch`, emit `session refused` (warn) with `reason` (`bad hello`, `not served`, `not a member`, `unexpected frame`), `territory` when known, and `identity` when authenticated, replacing the `unexpected request frame` warning; verify the admission tests pass and a non-member `Open` produces exactly one refusal event (in-process `tracing` test subscriber)
- [ ] 1.3 In `subscribe_session`, emit `subscription parked` (debug) after the initial reconcile and `subscription reaped` (info) with `reason` set by the three loop exits (`idle`, `lagged`, `closed`); verify `cargo test -p outpost --test liveness --test subscription` passes and the reaped-idle case emits `reason = idle`
- [ ] 1.4 At the `OrderRequest` arm, emit `order granted` (debug) with `position` or `order refused` (info) with `reason` (`not served`, `no fleet key`, `does not chain`), distinguishing the two arm-level refusals from `Replica::order` returning `None`; verify the ordering tests pass and a submission to a storage-only Station emits `no fleet key`
- [ ] 1.5 Convert the remaining string warnings in `station.rs` and the three in `vessel.rs`'s subscription worker to carry `territory` (and `error`) as fields; verify `rg "warn!\(\"" crates/outpost/src` finds no field-less warning that names a Territory in its text
- [ ] 1.6 Add `Station::live_sessions()` maintained by `serve_loop` (increment on accept, decrement when `handle` returns); verify a unit-level test that two parked subscriptions report 2 and 0 after both close

## 2. Binary events and the filter

- [ ] 2.1 Emit `holds loaded` (info, `count`, `duration_ms`) in `commands/station.rs` around the Station constructor, before `station running`; verify the readiness process test's log shows it with `count = 300` and a duration consistent with the observed window
- [ ] 2.2 Emit `shutting down` (info, `open_sessions`) on the signal and `station stopped` (info, `close_ms`, `timed_out`) after the close, replacing the two existing lines; verify `tests/signals.rs` reads `open_sessions = 1` in the parked-subscription case and `timed_out = false`
- [ ] 2.3 Document the floor: name `RUST_LOG` and the `outpost=info` default in `outpost station`'s long help (`main.rs` keeps the `EnvFilter` it has) and as a commented `env` entry in `deploy/kubernetes/base/statefulset.yaml`; verify a process test at the default floor sees every info and warn event for a registration, a refused registration, a reaped subscription, and a stop and no debug event, and one started with `RUST_LOG=outpost=debug` sees `territory re-registered`

## 3. The table test and the payload test

- [ ] 3.1 Write `crates/outpost/tests/events.rs` holding the spec's event table as data and driving the binary through register, re-register, refused register (a non-Owner), sync as a non-member, subscribe-and-idle (short liveness via a `--liveness` test knob or the in-process path), order, and SIGTERM, asserting each event's name, level, and required fields in the JSON log; verify every table row is asserted at least once and the test passes
- [ ] 3.2 Add the sentinel case: author a Notice whose plaintext is a fixed 32-byte sentinel, sync it with the floor at `RUST_LOG=outpost=trace`, and assert the sentinel, the hex of the ciphertext, and the envelope bytes appear in no log line; verify it passes, and that making a deliberately leaky `trace!` of a payload in a scratch build fails it
- [ ] 3.3 Run the in-process table against a `tracing` test subscriber for the events the binary cannot reach from outside (`subscription parked` under default liveness); verify it passes

## 4. Reference and hooks

- [ ] 4.1 Add the event table to the sync section of `.agents/skills/waypoints-architecture/references/architecture.md` with the metadata-only rule; verify it matches the spec's table row for row
- [ ] 4.2 Run `jj fix -s @` and the hook steps (clippy, cargo-test, treefmt, typos); verify all green
