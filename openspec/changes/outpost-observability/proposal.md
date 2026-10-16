## Why

A deployed Station is silent about its own work. The binary logs one line at start-up and one at shutdown; the library logs only failures, as unstructured strings. When the kind audit homed a Territory on a pod, the pod's log stayed empty, and the proof had to be a hold file on the node's disk. A commodore — the person who runs a Fleet, whatever its size — shipping JSON logs to a collector cannot answer the basic questions: which Territories were homed here and when, who was refused and why, which sessions were reaped, what the start-up scan cost. The first of those is also the earliest signal of the disk-filling problem that the deferred admission policy exists for.

A log line is also the wrong primary record. The Station already writes one durable fact about a registration into its own database, and naive logging would announce a second version of it to a stream nothing guarantees. The policy this change adopts: every Station keeps its own events table; the table is the foundation of an audit log; the log stream is a read-only reporter of it. `deployable-station` made the log format structured; this change gives it something durable to report.

## What Changes

- **Every Station keeps an events table** in its own database, beside sightings and registrations: a sequence number, a timestamp, the event kind, and its fields. An event is appended once the decision it records has taken effect — the hold exists, the session is reaped, the order was refused — so the table never claims something the Station did not do.
- **The log reports the table, read-only.** Each event line is produced from the row just written and carries its sequence number, so a collector can prove completeness by gaps and a line can be traced back to a row. If the write fails, a diagnostic warning says so and no event line is emitted.
- **Events and diagnostics are different things.** Events are decisions: a Territory homed, a registration or session refused with its reason, a subscription reaped with its reason, an order refused, the start-up scan with its count and duration, the shutdown with what was open. Diagnostics are failures and per-session detail — a repeat registration, a subscription parked, an order granted, an ingest that failed — and stay `tracing` only, now with fields. The spec's table lists the events; diagnostics are not a contract.
- **Events carry metadata only**: Territory ids, member ids, device public keys, counts, durations, reasons. Never payload bytes, envelopes, read keys, or key material. A test syncs a Notice with a sentinel plaintext and asserts the sentinel reaches neither the table nor the log.
- **Retention is a duration.** Events older than the retention period are pruned at start-up and on each append; the newest event is always kept so the sequence survives any idle stretch. Ninety days by default — three months — set with `--events-retain-days` (`OUTPOST_EVENTS_RETAIN_DAYS`), never below one day.
- **Refusals are coalesced.** A refusal is the one event a stranger can cause at will, so the first refusal of a kind from a source is recorded at once and further ones within a minute are counted into a single event with `count` and `since`. That bounds the record under a flood, not the flood: the Station still pays for every handshake. Admission rate-limiting — turning a source away before the handshake once it has been refused recently — is the long-term fix, and belongs with the admission-policy change, not here.
- **One floor, set by the environment.** The stream emits every event at or above a minimum level from `RUST_LOG`, defaulting to `outpost=info`; the table keeps every event regardless of the floor. Narrowing what is shown is for the collector, a UI, or `grep`, not a second knob on the Station. `outpost station --help` and the manifests name the variable and the default.
- **A read path for streaming.** The Station's database can be read from a sequence number forward — the foundation for a reader or a UI. No reader ships in this change.
- **The commodore.** The person who deploys and runs a Fleet is its commodore, in these documents and in the published specs; "operator" is left to Kubernetes.

## Capabilities

This change lands in **`sync-protocol`**, which carries the Station's operational contract. The events table and its stream are more of that contract: what a Station durably records about itself, what it tells its commodore, and what it must never tell.

### New Capabilities

None.

### Modified Capabilities

- `sync-protocol`: new requirements _A Station keeps an events table_ (the table, the event kinds and their fields, write-before-report, metadata-only, retention) and _Events are reported through the log as a read-only stream_ (the line from the row, the sequence number, diagnostics distinguished). The requirement _Station logs can be emitted as structured records_ is modified to state the floor and that it does not touch the table.

## Non-goals

- **A reader for the events table** — a CLI, an endpoint, a UI. The read path exists so one can be built; shipping one is developer and self-hoster convenience, not something a deployment waits on.
- **A metrics endpoint.** Counters and gauges are what the events would be derived into; a route with no exporter behind it is still a promise. The probe listener already exists to host one when the time comes.
- **Audit logging as a security control** — tamper evidence, signing, or retention guarantees beyond pruning. The table is an honest record on the Station's own volume; what a collector keeps of the stream is the commodore's policy. Flagged below because events name members and devices.
- **Replicating the events table.** It is per-Station, like sightings and registrations.
- **Vessel-side events in the app.** The app has its own logging; this change touches the Vessel only where its subscription worker already logs.
- **Changing what a Station refuses.** Reasons become visible; the policies do not move.
- **Admission rate-limiting.** Coalescing keeps a flood of refusals from filling the table, but every refused caller still gets a full handshake and an admission check. Turning a recently refused source away before that cost is paid is the long-term fix, and it belongs with the admission-policy change the hosted tier needs, where what a stranger may cost a Station is decided once.

## Impact

- `crates/outpost-hold/src/station_hold.rs` — an `EventRow` collection (sequence id, indexed timestamp, kind, JSON fields), `append_event`, `events_since`, `prune_events_before`, and the next sequence read at open.
- `crates/bedrock/src/database.rs` — `delete_where`, the sibling of `update_where` that pruning needs; a façade addition tested against the real engine, with no change to the storage spec's behavior.
- `crates/outpost/src/station.rs` — a record-then-report path called at each decision site, with explicit refusal and reap reasons where they are folded today; the string warnings become diagnostics with fields; `live_sessions()` for the shutdown event.
- `crates/outpost/src/vessel.rs` — the three subscription-worker warnings gain fields.
- `crates/outpost/src/bin/outpost/{main.rs,commands/station.rs}` — `--events-retain-days`, the scan and shutdown events, the floor named in the help.
- `crates/outpost/tests/` — every kind in the spec's table asserted as a row and as a line with a matching sequence; the sentinel; retention across a restart; the floor at default and at debug; a storage-less Station recording nothing.
- `.agents/skills/waypoints-architecture/references/architecture.md` — the event table and the term.
- `deploy/kubernetes/base/statefulset.yaml` — commented `RUST_LOG` and `OUTPOST_EVENTS_RETAIN_DAYS` entries.
- Privacy note, recorded in the spec: events name members and devices by public key, in the table on the volume and in whatever retains the stream; the hosted tier's retention policy has to account for both.
