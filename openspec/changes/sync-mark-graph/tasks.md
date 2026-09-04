Every task below must leave per-commit hooks green (clippy, cargo-test, treefmt) at its own commit. Persistence behavior is tested against the real engine — never by asserting generated SQL strings.

## 1. Preflight

- [x] 1.1 Confirm the `op-log-completeness` change has landed: every editor mutation emits `Maneuver`s, and the per-field `UpdateLandmarkTitle` (target `Title(MarkID)`) and `UpdateReading` (target `Reading(MarkID, FeatureID)`) variants exist. Do not start section 2 before this holds.
- [x] 1.2 Reseed the dev Territory from scratch and confirm no data backfill path is needed (existing Territory data is transient dev seed only).
      Verified 2026-09-04: `so.waypoints.dev/sync/` holds only `mnemonic.txt` + `sync.json`; `territory.db` has never been created on this machine, so there is no Territory data to migrate and the hold seeds from scratch on first creation.
- [x] 1.3 Record a baseline of the current dev Territory's Notice count and hold file size, for comparison against the measurement added in section 10.
      Baseline 2026-09-04: Notice count 0, hold file absent (0 bytes) — no dev Territory hold exists yet.

## 2. Arbitration key

- [x] 2.1 Settle the placement question first (design.md, Open Questions), then define the arbitration key type `(HLC, [u8; 32] author key bytes, NoticeID)` in `cartograph`'s sync layer — not `domain`, which must not depend on `outpost-protocol` — with a total `Ord` that compares lexicographically in that sequence.
- [x] 2.2 Implement derivation of the key from a parsed `EnvelopeHeader` alone, with no local-state inputs.
- [x] 2.3 Unit-test totality and determinism: equal clocks tie-break by author key then notice id; the comparison never returns `Equal` for distinct Notices; two independent derivations from the same header bytes agree.
- [x] 2.4 Property-test that sorting a shuffled vector of keys yields one canonical sequence regardless of input permutation.

## 3. HLC on the wire

- [x] 3.1 Give `Vessel` an HLC that ticks on author and applies `HLC::receive` when ingesting Notices, persisting the clock across restarts.
- [x] 3.2 Replace the hardcoded `timestamp: 0` in `Vessel::author` (`crates/outpost/src/vessel.rs`) with the ticked HLC value.
- [x] 3.3 Assert in tests that `validate()` outcomes are unchanged across header clock values (past, future, zero) — the field stays advisory at the protocol layer.
- [x] 3.4 Assert that mutating the header clock on a signed Notice breaks both signature verification and payload decryption (AAD binding).
- [x] 3.5 Test that a Vessel observing a Notice with a clock ahead of its own subsequently authors clock values exceeding it.

## 4. Fold core

- [x] 4.1 Define the payload codec for `waypoints.page.v1`: a batch of `Maneuver`s, with a round-trip test over every `Maneuver` variant confirming `#[serde(skip)]` undo-only fields deserialize to defaults and are never read.
- [x] 4.2 Implement the fold as a pure function over `(Notice header, batch of Maneuver)` pairs: sort by arbitration key, apply in order.
- [x] 4.3 Implement per-target resolution: group by `ManeuverTarget`, and for each target let the greatest key materialize.
- [x] 4.4 Test order independence directly — fold the same Notice set under many permutations and assert an identical resulting graph.
- [x] 4.5 Test that operations on distinct targets never interfere, and that concurrent operations on one target resolve to the higher key on every device.

## 5. Fold persistence

- [ ] 5.1 Add persistence for the winning arbitration key per `ManeuverTarget` (real-engine tests only). Note the key names `NoticeID`: this is where `store`'s scoped `outpost-protocol` dependency (design.md, Open Questions) is introduced, confined to sync-owned tables.
- [ ] 5.2 Reject on incremental apply any operation whose key is lower than the persisted key for its target, leaving the target untouched.
- [ ] 5.3 Test that a late-arriving older Notice does not clobber newer state.
- [ ] 5.4 Test that a full re-fold from empty reproduces the same graph _and_ the same persisted keys as incremental folding of the same Notice set.
- [ ] 5.5 Wire the incremental cursor to `TerritoryHold::notices_after(topo)` — `topo` selects which Notices are new locally; the arbitration key alone decides order. Add a test that two replicas with different `topo` assignments for the same Notices converge.

## 6. Publication

- [ ] 6.1 Add a durable outbound publish queue in `store` for locally applied, not-yet-published operations (real-engine tests).
- [ ] 6.2 Make local application and enqueue for publication a single unit in `EditorService`, so a crash between them cannot drop an edit.
- [ ] 6.3 Publish queued operations as `waypoints.page.v1` Notices during sync, batching per Notice, and clear them from the queue only on confirmed authorship.
- [ ] 6.4 Test the crash window: apply, simulate termination before publication, restart, and assert the operation is still queued and then publishes.
- [ ] 6.5 Test that re-publishing an already-published operation leaves every recipient in the same state (idempotence via the arbitration key).

## 7. Track selection

- [ ] 7.1 Add the write-side `MarkID → TrackID` index in `store` (real-engine tests).
- [ ] 7.2 Author Notices with `parents = track_heads(track_of(landmark))`, so a Landmark's history occupies one Track.
- [ ] 7.3 Define and implement the deterministic Track choice for operations concerning two Landmarks (e.g. `UpdateTerrain` moving a mark between Landmarks), derived from operation content so every device chooses identically. Test that two devices agree.
- [ ] 7.4 Assert the index is never read on the fold path — fold with the index absent, empty, and deliberately corrupted, and confirm an identical graph each time.
- [ ] 7.5 Test the degraded case: a stale index causes a Landmark's history to fragment across two Tracks, and the resulting state is still correct.
- [ ] 7.6 Test that divergence in one Landmark triggers resupply only for that Landmark's Track, via `TallyDigest::divergent`.

## 8. Dangling references

- [ ] 8.1 Add persistence for operations deferred on a missing `MarkID` (real-engine tests).
- [ ] 8.2 Defer, rather than apply or drop, any operation referencing a `MarkID` with no folded `CreateMark`.
- [ ] 8.3 Release deferred operations in arbitration-key order when the missing `CreateMark` folds.
- [ ] 8.4 Test that fold-then-create produces the same state as create-then-fold.
- [ ] 8.5 Expose the deferred-operation count so a permanently unresolved reference is observable rather than silent.

## 9. Transport

- [ ] 9.1 Switch `Station` to `Endpoint::bind_routable` and add configuration for the address it advertises to Vessels.
- [ ] 9.2 Switch `Vessel` endpoints to a routable bind so outbound traffic can leave the host, without requiring inbound reachability.
- [ ] 9.3 Give dev builds a stable OS-facing identity (signed binary / stable identifier) so the macOS application firewall authorizes once rather than per rebuild. Verify by rebuilding and rerunning without a new prompt.
- [ ] 9.4 Test a Vessel with no inbound reachability syncing against a Station at an advertised address, including live `NoticePush` delivery over the Vessel-opened session.
- [ ] 9.5 Confirm by inspection and test that `station.rs` still opens no outbound connections to Vessels.

## 10. Liveness

- [ ] 10.1 Add periodic liveness signalling to sessions in `outpost-fabric`, reusing existing session framing rather than adding a new wire message type where possible.
- [ ] 10.2 Add an idle threshold and stale-session reaping on the Station, releasing subscription and delivery state for sessions that stop signalling.
- [ ] 10.3 Test that a subscription whose Vessel vanishes without closing is reaped, and that an idle-but-signalling subscription is retained.
- [ ] 10.4 Add Vessel-side dead-session detection and reconnect, resuming from the local `topo` cursor.
- [ ] 10.5 Test sleep / network-change / cellular-drop reconnect: the Vessel receives Notices authored while disconnected, with no redundant resupply, and reaches the same state as a cold-start Vessel syncing once.

## 11. Measurement

- [ ] 11.1 Log Territory Notice count and hold file size on every sync, in normal operation rather than behind a diagnostic flag; include the deferred-operation count from 8.5.
- [ ] 11.2 Document the tripwire thresholds at which snapshots and compaction must be revisited, and where the measurement surfaces.

## 12. App surface

- [ ] 12.1 Expose sync state (last sync, pending publications, deferred operations, connection liveness) through `SyncService`.
- [ ] 12.2 Surface it in the Nuxt app via thin Tauri command wrappers, testing the crates rather than the Tauri harness.

## 13. Convergence acceptance

- [ ] 13.1 Two-Vessel convergence test through a Station: both converge on an identical Mark/Terrain graph.
- [ ] 13.2 Offline-divergence test: both Vessels edit while unable to reach each other, then both sync and converge.
- [ ] 13.3 Permuted-delivery convergence test at the integration level, proving order independence end to end and not only in the pure fold.
- [ ] 13.4 Cross-Landmark test: move a Content mark from one Landmark to another and confirm both Landmarks reflect the move on both devices.
- [ ] 13.5 Manual verification of the success condition on real hardware — desktop plus mobile, each with its own Vessel, against a self-hosted Station, including after offline divergence.
