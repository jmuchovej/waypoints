## Context

See proposal.md — Why. What the tree holds on 2026-10-14, assuming `fleet-roster` has landed:

- **Sync is already peer-symmetric at the session layer.** `outpost-fabric`'s `initiate` and `serve_gated` run the Tally and Resupply state machines over any `SyncPeer`; `Replica` implements `SyncPeer`. A Vessel syncs by `connect` then `initiate`; a Station serves by `accept` then `serve_gated`. Nothing in the session layer knows which role is which, so a Station can be an initiator with no new protocol for the reconcile itself.
- **What is missing for a pull.** A puller must learn which Territories a peer serves (`Station.territories` is private and never sent) and must bootstrap a hold for one it lacks (`Replica::bootstrap_with_fleet(path, acl_log, fleet)` needs the ordered log first; the ACL phase of a sync assumes a replica already exists to ingest into). Hence two requests: the Territory list, and one Territory's ordered log.
- **Admission.** `dispatch` admits `Open` under `SyncPolicy::Members` only with a `Hello` (identity plus device cert); a Station has neither. After `fleet-roster`, the Station holds a `FleetConfig` whose `station(remote_key)` answers "is this caller a rostered Station" from the transport-authenticated key alone.
- **The pin.** `tests/transport.rs::station_source_opens_no_outbound_connections` counts `.connect(` in `station.rs` and requires zero. The spec clause it enforces says "SHALL NOT initiate outbound connections to Vessels".
- **Keys per pod.** A StatefulSet pod knows its ordinal only through its name. Kubernetes expands `$(POD_NAME)` in `volumeMounts[].subPathExpr`, so a Secret holding `outpost-0.key … outpost-N-1.key` can be mounted as `/keys/station.key` per pod with no shell in the image; the same works for a per-pod advertise file from a ConfigMap. Environment values cannot be indexed that way, which is why the advertised socket moves to a file.
- **Orderer.** `--fleet-key-file` turns the role on; after `fleet-roster` a Station refuses to start with a fleet key if its roster entry lacks the orderer role. A uniform StatefulSet template therefore cannot give pod 0 the key and the others not; a second StatefulSet can.
- **The app.** `cartograph` dials one configured Station for everything, ordering included. It keeps working if that Station is a full Station: storage plus orderer.

## Goals / Non-Goals

**Goals:**

- Convergence: any Territory on any rostered Station reaches every rostered Station within one interval, Vessels or no Vessels.
- The accept-only property survives as a pinned invariant with a narrower, exact statement.
- Scaling to N within the pool is a `replicas` edit; nothing re-signed, nothing re-advertised.

**Non-Goals:**

- Beyond the proposal's: no attempt to make a replication cycle cheaper than one session per Territory per peer; no replication of the Station's own database (sightings and registrations are per-Station observations).

## Decisions

### D1 — The replicator is a module beside the Station, and the pin becomes two

**Chosen** (2026-10-14). `crates/outpost/src/replication.rs` defines `Replicator { station: Arc<Station>, roster: FleetConfig, endpoint: Endpoint, interval }` with `run()` — the loop — and `cycle()` — one pass. It dials with the Station's own identity key through its own `Endpoint` (bound routable, so the source socket can leave the host), never through the Station's serving endpoint. `station.rs` keeps zero `.connect(`, so the existing pin holds verbatim. A second pin asserts that `replication.rs` obtains every address it dials from a `FleetConfig` entry: statically, the module has no `SocketAddr` or `EndpointAddr` constructor other than `entry_addr(&StationEntry)`; behaviourally, the convergence suite runs a cycle with a non-rostered Vessel endpoint listening and asserts it never accepts.

**Rejected.** _Replication inside `Station`_ — would void the pin and tangle the serve loop with a client loop. _A separate binary_ — two processes per pod for one loop, and the replicator needs the Station's replicas in-process to ingest into.

### D2 — Pull, in three steps per peer

`cycle()` for each storage entry in the roster other than self: connect; `TerritoriesRequest` → the peer's served ids; for each id the Station does not hold, `LogRequest { territory }` → `LogResponse { log }`, verify the log folds and that `id == hash(log[0])`, `Replica::bootstrap_with_fleet(path, log, fleet)`, insert into `territories`; then for every id the peer serves and the Station now holds, `Open { territory }` followed by `initiate` over the Station's replica for it. Peers are visited sequentially with a bounded per-peer time; a connect failure logs `peer unreachable` and moves on. Triggers: `run()` waits for the Station's holds to be loaded (the readiness flag), runs a cycle, then sleeps `interval` with jitter; `Station::register` sends a nudge on a `watch` channel after a new hold is created, and the loop wakes to run a cycle early.

**Why the full log and not the ACL phase.** The ACL phase ingests into an existing replica; bootstrapping needs the root before a replica can exist. One request that returns `Vec<OrderedRecord>` is simpler than teaching the session to create holds.

### D3 — Roster-peer admission is a check on the transport key, not a policy variant

`dispatch` computes `let peer = self.roster.as_ref().and_then(|r| r.station(&session.remote_key())).filter(|e| e.roles.storage)` before matching the first frame. With `peer.is_some()`: `Open` is admitted without a `Hello` and the two-phase gate returns `true` for both phases; `TerritoriesRequest` and `LogRequest` are answered; `Register` and the rest behave as for any caller. Without it, the two new requests are refused with `Ack { ok: false }`. `SyncPolicy` keeps its two arms: the doc comment's promised "roster-peer arm" turns out to be a cross-cutting check rather than a third policy, since a roster peer must be admitted under `Members` and `Everyone` alike.

### D4 — Replicated holds are fleeted; the orderer's proofs are the only authority

Every hold a Station creates from a fetched log is `bootstrap_with_fleet` with the roster's Fleet identity (holds created by `Register` already are, after `fleet-roster`). Replication therefore cannot introduce an ACL record the orderer did not countersign: a forged proof is refused at ingest exactly as on a Vessel. Notices need no such check; they carry their own signatures and are ciphertext.

### D5 — The orderer is a second StatefulSet; the app points at it

`deploy/kubernetes/base` gains `statefulset-orderer.yaml`: one replica, its own key Secret, the fleet key Secret, `--roster`, a per-Station Service `outpost-orderer`; its roster entry has `storage` and `orderer`. The storage StatefulSet `outpost` has `replicas: N`, no fleet key mount, and entries with `storage` only. The orderer stores and replicates like every other Station, so an app configured with the orderer's socket keeps working unchanged — ordering, syncing, subscribing — while the storage Stations hold copies. When `vessel-multi-station` lands, the app learns to find the orderer by role and the storage Stations by hashing; nothing here moves.

**Why not pod 0 as the orderer.** The template is uniform and `fleet-roster` makes a fleet key without the role a start-up refusal. A second StatefulSet is three resources and no special cases.

### D6 — The identity pool is one Secret and one ConfigMap indexed by pod name

`outpost-station-keys` (Secret) holds `outpost-0.key … outpost-{N-1}.key`; the storage StatefulSet mounts it with `subPathExpr: $(POD_NAME).key` at `/keys/station.key`, `POD_NAME` from the downward API. `outpost-advertise` (ConfigMap) holds `outpost-0 … outpost-{N-1}`, one socket per line, mounted the same way at `/config/advertise`; the Station gains `--advertise-file` (`OUTPOST_ADVERTISE_FILE`), read like `--advertise` (empty means none). `deploy/justfile` gains `fleet-keys N` (N station keys plus the orderer's and the fleet key, into the scratch keys dir), `fleet-roster` (derives each identity with `outpost station id`, assigns sockets from the per-Station Services' addresses — or the kind overlay's host ports — and signs), and `kind-apply` grows to create the pool Secret, the advertise ConfigMap, and the roster ConfigMap. Per-ordinal Services: `outpost-0 … outpost-{N-1}` and `outpost-orderer`, each selecting `statefulset.kubernetes.io/pod-name`; `LoadBalancer` in the base, NodePort in the kind overlay with host ports 6782 (orderer), 6783, 6784 (storage 0 and 1) in `kind-config.yaml`.

### D7 — The audit grows to three Stations

`tests/audit.rs` gains `fleet_converges` driven by `OUTPOST_AUDIT_ROSTER`: register and sync with the orderer, then poll each storage Station until it serves the Territory and holds the Notice, reporting the lag. The kind run in the tasks then deletes one storage pod _and its claim_ (so the volume is gone) and asserts the replacement backfills without a Vessel.

## Risks / Trade-offs

- [One session per Territory per peer per cycle: 300 Territories across 3 Stations at 60 s is 600 sessions a minute per Station] → Each is a Tally round-trip when nothing changed; QUIC connection reuse within a cycle (one connection per peer, many streams) is the first optimisation and is allowed by `Endpoint::connect` returning a session per stream. Measured in the audit; the interval is the knob.
- [A replication cycle and a Vessel sync can ingest the same Notice concurrently] → Ingest is idempotent by design (`SyncPeer::ingest` and the hold's primary keys); the live-push bus already tolerates duplicates.
- [Two new wire messages, as with the roster request] → Same terms: pre-1.0, roster-peer-only, refused by older Stations.
- [A roster peer that lies about which Territories it serves can make a Station bootstrap junk] → The log must fold and hash to the id, and every non-root record needs a valid proof; a rostered Station is trusted to that extent and no further, which is what being in the roster means.
- [`subPathExpr` keys must exist for every ordinal up to `replicas`] → A pod whose ordinal has no key fails to mount and never starts; `fleet-keys N` and `replicas ≤ N` are documented together in the kustomization, and the kind recipe checks it.
- [Budget: 15 to 25 hours] → Groups 1 to 3 (protocol, replicator, in-process convergence suite) are shippable without the manifests; groups 4 to 6 can be the following sprint's first item.

## Migration Plan

1. Land the two messages and roster-peer admission with in-process tests.
2. Land the replicator and the convergence suite; the second pin.
3. Land `--replicate-every`, `--advertise-file`, and the shutdown wiring.
4. Land the manifests, the recipes, and the kind overlay at three Stations.
5. Run the audit; record lag, cycle cost, and the backfill.
6. Rollback per step; a Station without a roster never replicates, so every step is inert for the one-Station Fleet.

## Open Questions

- The interval's default. 60 s is a guess that trades convergence lag against cycle cost; the audit's numbers decide it, and the option exists either way.
- Whether a Station should also replicate from the orderer's `TerritoriesResponse` the Territories it serves but the peer does not (push-style announce). Not needed for convergence — every Station pulls from every peer — but it would shorten the first cycle for a brand-new Station with many peers.
