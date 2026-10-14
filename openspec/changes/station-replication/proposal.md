## Why

A Fleet is meant to hold every Territory on every Station — the Station module's docs say so — but no Station ever talks to another. Today a second replica would be a second, disjoint Fleet: a Territory lands on whichever Station its owner registered with, and a Station added later holds nothing until an owner happens to sync with it, which an offline owner never does. That is why `deployable-station` pinned `replicas` at one and why the hosted tier cannot yet sell availability.

This change makes Stations replicate. Each Station in a roster pulls every Territory from its roster peers — the ones it already homes, to converge, and the ones it has never heard of, to backfill — so killing a Station loses nothing and a new one fills itself. On that footing the reference manifests grow to a Fleet of N storage Stations and one orderer, with the identities minted ahead of time so that scaling is a `replicas` edit and never a roster re-sign.

It depends on `fleet-roster`: replication is admitted by roster, and the manifests sign one.

## What Changes

- **Stations pull from roster peers.** A Station started with a roster runs a replicator: at start-up once its holds are loaded, on an interval, and immediately after it homes a new Territory, it dials each storage Station in the roster and, for every Territory the peer serves, either syncs the Territory it already holds or fetches the peer's ACL log to bootstrap a fleeted hold and then syncs. The ordinary reconcile does the work; the Station is the initiator for once.
- **Roster peers are admitted by key, not by handshake.** A caller whose authenticated identity is a storage Station in the roster is admitted to sync any Territory and to two new requests: the list of Territories the Station serves, and the ordered ACL log of one. Vessels never see these; a Station without a roster never answers them.
- **The accept-only invariant narrows and stays pinned.** The Station never dials Vessels; it dials only addresses from its roster. The replicator lives beside the Station, not in it, so the existing pin on the Station's source holds verbatim, and a second pin covers the replicator.
- **An identity pool makes scaling a number.** The roster lists N storage Stations whose keys are generated ahead of time; the StatefulSet runs up to N replicas, each pod finding its key and its advertised socket by its own ordinal. A pod that is scaled away is simply an unreachable roster entry; nothing is re-signed on a scale event.
- **The orderer is its own singleton.** A one-replica StatefulSet holds the fleet key and the orderer role, and stores and replicates like every other Station, so an app configured with its address keeps working exactly as today while the storage Stations behind it converge. Only it mounts the fleet key.
- **Every Station has its own socket.** Per-ordinal Services route each Station's UDP port to its own address; the identity check a Vessel performs on connect makes a shared load-balanced address useless, so none is offered.
- **Manifests, recipes, and an audit for a three-Station kind Fleet**: two storage Stations and the orderer converge on a Territory, one is killed and loses nothing, and a fresh one backfills.

## Capabilities

**`sync-protocol`** carries replication, roster-peer admission, and the narrowed accept-only rule. **`distribution/station-deployment`** carries the N-Station manifests, the identity pool, the orderer singleton, and per-Station addressing.

### New Capabilities

None.

### Modified Capabilities

- `sync-protocol`: the requirement _A Station is reachable at an advertised address_ changes its accept-only clause to "accepts from Vessels; dials only roster peers". New requirements _Stations in a Fleet replicate every Territory_ and _Roster peers are admitted by identity_.
- `distribution/station-deployment`: the requirement _Reference manifests describe a one-Station Fleet that survives pod replacement_ becomes a Fleet of N storage Stations and one orderer with an identity pool. New requirement _Each Station has its own advertised socket_.

## Non-goals

- **Vessel-side failover and sub-roster selection.** The app keeps one configured Station (the orderer's address, in the reference layout). Choosing storage Stations by rendezvous hashing, failing over among them, and finding the orderer by role is the next change, `vessel-multi-station`; it needs this one's convergence to be worth anything.
- **Relay-assisted reachability.** Per-Station public sockets are the no-relay path; the relay spike is its own change, `fleet-relay`, and it changes only how an entry's address is written.
- **Live Station-to-Station push.** Replication converges on an interval and on registration; a Notice is on every Station within one interval. Subscriptions between Stations can shorten that later without changing the admission or the messages.
- **Capacity sharding.** Every Station holds everything; storage per Station equals the Fleet's total. Sharding is a different design.
- **Helm, an operator, HPA, PodDisruptionBudgets.** The kustomize base grows a replica count and per-ordinal resources; a chart arrives when there are values to vary.
- **Blob role.** Still absent from every roster.

## Impact

- `crates/outpost-protocol/src/sync.rs` — `TerritoriesRequest`/`TerritoriesResponse` and `LogRequest`/`LogResponse`; a wire addition on the same terms as the roster request.
- `crates/outpost/src/replication.rs` — new: the replicator, its triggers, and its one rule (addresses come from the roster).
- `crates/outpost/src/station.rs` — roster-peer admission in `dispatch` for `Open`, the two new arms, a registration hook that nudges the replicator, `Replica::bootstrap_with_fleet` from a fetched log.
- `crates/outpost/src/bin/outpost/commands/station.rs` — `--replicate-every` (`OUTPOST_REPLICATE_EVERY`, default 60 s), `--advertise-file`; the replicator spawned beside the serve loop and stopped on shutdown.
- `crates/outpost/tests/` — the convergence suite (two Stations plus an orderer in-process over loopback), the backfill case, the second accept-only pin, and the audit harness extended for N Stations.
- `deploy/kubernetes/base` — `replicas` parameterised, per-ordinal Services, the identity-pool Secret and advertise ConfigMap mounted by `subPathExpr`, the orderer StatefulSet; `deploy/justfile` recipes to mint N keys and sign the roster; the kind overlay at three Stations.
- The sprint: the seed estimated this phase at 15 to 25 hours, more than half a sprint. The design orders the work so that replication without the manifests (groups 1 to 3) is shippable on its own if the budget runs out.
