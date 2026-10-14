Depends on `fleet-roster`. Groups 1 to 3 are shippable on their own; groups 4 to 6 can follow in a later sprint if the budget runs out. Tasks marked `[audit]` end in an observation made outside CI; record it when ticking.

## 1. Protocol and admission

- [ ] 1.1 Add `SyncMessage::{TerritoriesRequest, TerritoriesResponse { territories }, LogRequest { territory }, LogResponse { log }}` to `crates/outpost-protocol/src/sync.rs` with the wire-compatibility note; verify the protocol crate's round-trip tests cover all four
- [ ] 1.2 In `Station::dispatch`, compute the roster-peer check from the transport key per design.md D3, admit `Open` without a handshake for a storage roster peer with both gate phases open, answer the two new requests only to roster peers, and refuse them otherwise; verify integration tests: a rostered Station opens a Territory on another without `Hello` and both phases serve; an authenticated Vessel's `TerritoriesRequest` is refused; an unrostered Station without `Hello` is refused like an anonymous caller
- [ ] 1.3 Make `Station::register` bootstrap fleeted holds (it does after `fleet-roster`) and add `Station::bootstrap_from_log(territory, log)` for the replicator, verifying the fold and `territory == hash(log[0])`; verify a unit test that a log whose id does not match is refused and that a non-chaining log is refused

## 2. The replicator

- [ ] 2.1 Add `crates/outpost/src/replication.rs` with `Replicator::{new, run, cycle}` per design.md D2: its own routable endpoint on the Station key, sequential peers with a bounded per-peer time, `TerritoriesRequest` → bootstrap-then-sync or sync, `peer unreachable` skipped and retried; verify an in-process convergence test with two rostered Stations over loopback where a Territory registered on one is served by the other after one cycle with its Notice
- [ ] 2.2 Add the registration nudge (`watch` channel from `Station::register` on a new hold) so a cycle runs promptly after a registration; verify the convergence test observes the Territory on the peer well before the interval would have elapsed
- [ ] 2.3 Add the backfill test: three Stations, Territories registered on two of them by owners that are then dropped, a third Station started with the roster converges on all of them with no Vessel involved; verify it passes, and that the third Station's holds are fleeted (a forged proof over sync is refused)
- [ ] 2.4 Add the down-peer test: one of three peers is stopped mid-way; verify the cycle completes for the others, the Station keeps serving a Vessel during the cycle, and the next cycle picks the restarted peer up
- [ ] 2.5 Add the second accept-only pin in `tests/transport.rs`: statically, `replication.rs` constructs addresses only from roster entries (one constructor, asserted by source inspection); behaviourally, a non-rostered Vessel endpoint listening during a cycle never accepts a connection; verify both pass and `station_source_opens_no_outbound_connections` still counts zero
- [ ] 2.6 Add the plaintext test: a Notice replicated between two Stations is byte-identical in both holds and neither Station holds a read key; verify it passes

## 3. The binary

- [ ] 3.1 Add `--replicate-every` (`OUTPOST_REPLICATE_EVERY`, default 60 s) and spawn the replicator beside the serve loop when a roster is held, waiting for readiness and stopping on shutdown; verify a process test with two binaries on ephemeral ports sharing a roster converges on a Territory, and that `station stopped` is reached within the close bound with a cycle in flight
- [ ] 3.2 Add `--advertise-file` (`OUTPOST_ADVERTISE_FILE`), one socket per line, empty meaning none, merged with `--advertise`; verify unit tests on the parser and a process test that the published address carries the file's sockets
- [ ] 3.3 Run `cargo test -p outpost -p outpost-fabric -p outpost-protocol` and clippy; verify all green

## 4. Manifests and recipes

- [ ] 4.1 Add `statefulset-orderer.yaml` (one replica, its own key Secret, the fleet key, `--roster`, Service `outpost-orderer`) and make the storage StatefulSet `replicas: N` with the pool Secret and advertise ConfigMap mounted by `subPathExpr: $(POD_NAME)…` and `POD_NAME` from the downward API; verify `just deploy::lint` passes at N = 1 and N = 3 (a patch sets `replicas`)
- [ ] 4.2 Add per-ordinal Services `outpost-0 … outpost-{N-1}` selecting `statefulset.kubernetes.io/pod-name` (LoadBalancer in the base; NodePort in the kind overlay with host ports 6782 orderer, 6783 and 6784 storage in `kind-config.yaml`); verify the rendered overlay has no Service selecting more than one pod
- [ ] 4.3 Add `deploy/justfile` recipes `fleet-keys N` (N storage keys, the orderer key, the fleet key), `fleet-roster` (derive identities with `outpost station id`, assign sockets, sign with `outpost fleet roster sign`), and extend `kind-apply` to create the pool Secret, the advertise ConfigMap, and the roster ConfigMap; verify `just deploy::kind-apply` on a fresh cluster brings up three Ready pods, each logging a distinct identity that matches its roster entry
- [ ] 4.4 Rewrite the kustomization's header for the N-Station layout: the pool, `replicas ≤ N`, the per-Station sockets, the orderer; verify it reads correctly against the rendered output

## 5. Audit on kind

- [ ] 5.1 Add `fleet_converges` to `tests/audit.rs` driven by `OUTPOST_AUDIT_ROSTER`: register and sync with the orderer, then poll each storage Station until it serves the Territory and holds the Notice, printing the lag; verify it passes against the three-Station kind Fleet and record the lag
- [ ] 5.2 `[audit]` Delete one storage pod together with its claim; verify the Territory is still served by the others, the replacement pod starts on the same pooled identity, backfills the Territory with no Vessel involved, and the harness finds the Notice on it; record the backfill time
- [ ] 5.3 `[audit]` Scale the storage StatefulSet from 3 to 1 and back to 3; verify no roster change, the returning pods converge, and `kubectl get events` shows no probe restarts
- [ ] 5.4 `[audit]` With 300 Territories (`register_many` against the orderer) record a cycle's duration and the number of sessions per cycle from the events, and the lag to full convergence on the storage Stations; verify the numbers are in the design's risk entry and decide the interval's default from them

## 6. Close-out

- [ ] 6.1 Update the Station module docs (replication now exists; the roster-peer "arm" is a check), the sync section of `.agents/skills/waypoints-architecture/references/architecture.md`, and the design's open questions from the audit; verify the docs no longer promise a future replication
- [ ] 6.2 Run `jj fix -s @`, the hook steps, `just check-generated`, and `just deploy::lint`; verify all green
