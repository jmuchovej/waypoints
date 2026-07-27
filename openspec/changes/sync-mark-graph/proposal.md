## Why

Outpost works end to end — identity, ACL, encrypted notices, Territory/Station/Vessel roles, QR pairing, device registry, `sync_with`, `subscribe` — and `crates/cartograph/src/sync/service.rs` already drives it from the app. But **none of the user's actual notes sync.** The `Maneuver` op enum is used only for local undo/redo, `EditorService` holds zero sync references, and `payload_type: "waypoints.page.v1"` appears only inside outpost's own tests: there is no producer anywhere in app code. Waypoints is a local-first app whose sync engine carries no user data.

This change closes exactly that gap: it makes editor `Maneuver`s the payload of outpost Notices, defines the merge law that turns concurrent Notices back into one Mark/Terrain graph, and fixes the two transport gaps (loopback-only binds, no connection liveness) that stop two real devices from reaching each other.

## What Changes

**Op transport — the missing producer and consumer**

- `EditorService` mutations publish their `Maneuver`s as outpost Notices under `payload_type: "waypoints.page.v1"`, encoding a batch of ops per Notice.
- A consumer folds inbound Notices back into the Mark/Terrain graph via the existing `Maneuver::apply` path, so remote ops and local ops converge on one write path.
- Local application and publication are one unit: an op that is applied locally is durably queued for publication, so a crash between the two cannot silently drop a user edit.

**Merge law — per-target last-write-wins under a content-derived total order**

- Arbitration key per `ManeuverTarget` is `(hlc, author_verifying_key_bytes, notice_id)`. Every component lives in the signed `EnvelopeHeader`, so every device derives an identical total order from content alone.
- `EnvelopeHeader.timestamp` — currently hardcoded to `0` in `Vessel::author`, documented "advisory only, never ordering", signed and AAD-bound — is populated from the existing `domain::sync::hlc` HLC. Its advisory status in the protocol is unchanged; ordering is an app-layer reading of a field the protocol does not interpret.
- The fold sorts by that key and applies in order. The winning key per target is persisted, so a late-arriving older Notice is rejected on incremental apply rather than clobbering newer state.
- Order-independence comes from sorting a content-derived total order, **not** from algebraic CRDT design.

**Fold scope — territory-wide**

- The fold reads every Notice in the Territory, not one Track. Ops legitimately span two Landmarks: moving a Content mark from L1 to L2 is one `UpdateTerrain { new_src }` concerning both, but it lands in exactly one Track. Per-Track folding would be wrong even with perfect Tracks.
- `topo` is a local per-replica arrival counter, so it is used only as a "what is new locally" cursor for incremental apply — never as the order to fold by.

**Track granularity — one Track per Landmark**

- Notices are authored with `parents = track_heads(track_of(landmark))`, which is the app's only lever over outpost's structural Track assignment.
- The `MarkID → TrackID` index is **write-side only**: consulted solely to choose `parents`, never on the read/fold path. A lost or corrupt index therefore cannot corrupt state — worst case is a Landmark whose history fragments across two Tracks (degraded partitioning, correct state).
- Tracks are a sync-partitioning mechanism, not a semantic boundary: `TallyDigest::of_heads` keys on `TrackID`, so per-Landmark Tracks mean divergence triggers resupply only for the Landmarks that actually diverged.

**Payload-level dangling references**

- Outpost parks orphans at the _notice-parent_ level (`park_pending`/`pending_on`/`unpark`). An op referencing a `MarkID` whose `CreateMark` has not folded yet is an app-level concern and is currently unhandled. This change defines deferral and release for those references.

**Transport reachability**

- Stations bind via `Endpoint::bind_routable` (which exists today with zero callers) and advertise a public address; the operator self-hosts the Station.
- Vessels get a routable bind so outbound packets can route at all. This is a _source-socket_ fix, not a reachability fix: Stations are accept-only (`bind_identity` plus an `accept()` loop, no outbound `.connect(` in `station.rs`), and `NoticePush` travels Station→Vessel over the session the Vessel opened via `Subscribe`. Vessels need no inbound reachability.
- Dev builds get stable identifiers / signed binaries so the macOS application firewall authorizes once instead of on every recompile — the ergonomics problem that motivated the loopback-only bind in the first place.

**Connection liveness**

- There is no heartbeat, keepalive, or idle timeout anywhere in `outpost-fabric` or `station.rs`, and nothing detects or reaps a stale subscription. A phone that sleeps, changes networks, and drops cellular makes this a real requirement. This change adds heartbeat, stale-session reaping, and mobile reconnect.

**Log growth measurement**

- No snapshots and no compaction (see Non-goals); instead, sync logs Notice count and hold file size, and a documented tripwire records when unbounded growth must be revisited.

## Capabilities

`openspec/specs/` is currently empty — specs emerge from differentials and are never backfilled — so both domains this change touches are seeded here rather than amended.

### New Capabilities

- `sync`: app-side op-log sync — publishing editor `Maneuver`s as Notices, the per-target LWW fold and its arbitration key, the write-side `MarkID → TrackID` index, payload-level dangling-reference deferral, offline divergence and reconvergence, and log-growth measurement.
- `sync-protocol`: engine-side contracts this change relies on or alters — HLC population of the advisory `EnvelopeHeader.timestamp`, routable Station/Vessel binds and Station address advertisement, and connection liveness (heartbeat, stale-session reaping, reconnect).

Per the domain registry, `sync` may cite `sync-protocol` guarantees; the reverse is never true, and the deltas here respect that direction.

### Modified Capabilities

None — no spec exists yet for either domain.

## Impact

**Depends on** the in-flight `op-log-completeness` change, which makes every editor mutation produce `Maneuver`s and adds the per-field `UpdateLandmarkTitle` (target `Title(MarkID)`) and `UpdateReading` (target `Reading(MarkID, FeatureID)`) variants. This change assumes those exist and does not duplicate that work; it cannot land before them, because publishing an incomplete op log would sync a graph that silently diverges from local state.

**Code**

- `crates/cartograph` — `sync/service.rs` (publish/fold driver, liveness, measurement), `editor/service.rs` (mutations emit for publication), new fold and Track-index modules.
- `crates/domain` — `sync/hlc.rs` becomes load-bearing rather than unused; arbitration-key types.
- `crates/store` — persistence for the per-target winning arbitration key, the write-side `MarkID → TrackID` index, the outbound publish queue, and deferred dangling-reference ops.
- `crates/outpost` — `vessel.rs` (`author` populates `timestamp` from the HLC instead of `0`), `station.rs` (routable bind, public address, session reaping).
- `crates/outpost-fabric` — `endpoint.rs` (routable binds for Vessels), heartbeat/idle-timeout.
- `waypoints/src-tauri` + Nuxt app — surfacing sync state and reconnect; commands stay thin wrappers.

**Data**

- No backfill. The only existing Territory data is transient dev seed, which is reseeded rather than migrated.

**Wire format**

- `EnvelopeHeader.timestamp` changes value (`0` → HLC) but not shape; no wire break. `TextPayload` is already a tagged enum, so char-level merge can land later without a wire break.

## Non-goals

- **No snapshots and no compaction.** `Vessel::author` hardcodes `is_snapshot: false` and `snapshot_base` is deferred in outpost itself. Unbounded log growth is accepted here; only measurement and a tripwire are in scope.
- **No CRDT and no char-level text merge.** Per-target LWW under a content-derived total order is an LWW-Register and converges. A general CRDT buys nothing for the genuine same-field-same-instant case.
- **No Vessel-to-Vessel sync.** All sync flows through a Station. V2V remains a core architectural goal, so nothing here is designed in a way that forecloses it — but relay and DNS discovery (absent from `presets::Minimal`) matter only for V2V and stay out of scope.
- **No NAT traversal, hole-punching, or relay.** Vessels need no inbound reachability, so none of this is required to reach the success condition.
- **No multi-user sharing** beyond what outpost's ACL already provides.
- **No data backfill or migration** of existing Territory data.

## Success Condition

Two devices — desktop and mobile, each with its own Vessel — converge on the same Mark/Terrain graph through a self-hosted Station, including after offline divergence. Convergence tests exercise **permuted delivery order** and prove the fold is order-independent. Persistence tests run against the real engine and are never string-asserted SQL; every task leaves per-commit hooks green.
