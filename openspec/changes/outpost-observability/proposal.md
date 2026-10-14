## Why

A deployed Station is silent about its own work. The binary logs one line at start-up and one at shutdown; the library logs only failures, and those as unstructured strings. When the kind audit homed a Territory on a pod, the pod's log stayed empty, and the proof had to be a hold file on the node's disk. An operator shipping JSON logs to a collector cannot answer the basic questions: which Territories were homed here and when, who was refused and why, which sessions were reaped, what the start-up scan cost. The first of those is also the earliest signal of the disk-filling problem that the deferred admission policy exists for.

The deployable Station made the log format structured; this change makes it say something.

## What Changes

- **The Station emits structured events for its operations**, with stable field names and stated levels: a Territory homed (and a re-registration, at a quieter level), a registration refused with its reason, a session refused with its reason, a subscription parked and reaped, an order request granted or refused, the start-up scan with its count and duration, and the shutdown with what was open. The library emits them through `tracing`; the binary's two formats render them.
- **Events carry metadata only.** Territory ids, member ids, device keys, counts, and durations — things the Station already stores. Never payload bytes, read keys, or key material. A test authors a Notice with a sentinel and asserts the sentinel never reaches the log.
- **The existing failure logs become events too**: the string-formatted warnings in the Station and the Vessel's subscription worker gain fields so a collector can filter on a Territory rather than grep for a phrase.
- **One floor, set by the environment.** The Station emits every event at or above a minimum level and nothing below it. The floor comes from `RUST_LOG`, as today, defaulting to `outpost=info`; the table assigns levels so that the default floor is the whole operational record — refusals at warn, decisions an operator acts on at info, per-session detail at debug. Narrowing what is shown further is for the collector, a UI, or `grep`, not a second knob on the Station. `outpost station --help` and the manifests name the variable and the default.
- **The architecture reference** gains the event list beside the sync section, since the events are part of the Station's contract.

## Capabilities

This change lands in **`sync-protocol`**, which carries the Station's operational contract (reachability, keys, signals, probes, logs from `deployable-station`). Events are more of that contract: what a Station tells its operator, and what it must never tell.

### New Capabilities

None.

### Modified Capabilities

- `sync-protocol`: new requirement _A Station reports its operations as structured events_ (the event list, levels, fields, and the metadata-only rule). The requirement _Station logs can be emitted as structured records_ is modified to state the minimum-level floor, how it is set, and what the table's levels promise.

## Non-goals

- **A metrics endpoint.** Counters and gauges on `/metrics` are the natural next step once there is something to graph; the events here are what a metric would be derived from, and a route with no exporter behind it is still a promise. The probe listener already exists to host it when the time comes.
- **Audit logging as a security control**, with tamper evidence or retention guarantees. These are operator-facing events; what a collector keeps, and for how long, is the operator's policy — flagged below because the events name members and devices.
- **Vessel-side events in the app.** The app has its own logging; this change touches the Vessel only where its subscription worker already logs.
- **Changing what a Station refuses.** Reasons become visible; the policies do not move.

## Impact

- `crates/outpost/src/station.rs` — events at registration (new hold, idempotent re-register, refusal with reason), session admission refusals, subscription park and reap, order grant and refusal, with fields; the existing `warn!` strings become events with fields.
- `crates/outpost/src/vessel.rs` — the three subscription-worker warnings gain fields.
- `crates/outpost/src/bin/outpost/{main.rs,commands/station.rs}` — the scan event (count, duration) around the Station constructor, the shutdown event (open sessions, close duration), and the long help naming `RUST_LOG` and its default.
- `crates/outpost/tests/` — a process-level test over the JSON log: the registration event appears with the Territory id after a register, the refusal event after a non-Owner registration, and a sentinel payload never appears; a unit test that every event name in the spec is emitted somewhere (a table in the test, not a registry in the code).
- `.agents/skills/waypoints-architecture/references/architecture.md` — the event list.
- Privacy note for the hosted tier, recorded in the spec: events name members and devices by public key, which a collector then retains; the hosted tier's retention policy has to account for that.
