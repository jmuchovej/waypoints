## Context

See proposal.md — Why. This section records only the current-state facts that shape the approach; each was verified against the code at proposal time (2026-07-30).

**What already exists**

- `domain::HLC` (`crates/domain/src/sync/hlc.rs`; the `sync::hlc` module is `pub(crate)`, re-exported at the crate root) — hybrid logical clock packing 48 bits physical ms and 16 bits logical into one `i64`. The derived `Ord` matches `(physical, logical)` because logical occupies the low bits and physical is non-negative. Unit-tested with an injectable `Clock`. Nothing consumes it today.
- `Maneuver` (`crates/cartograph/src/editor/maneuver.rs`) — already documented as a wire type ("ops travel between devices inside `Envelope`s") with `#[serde(skip)]` on undo-only fields, and `ManeuverTarget` already carries the comment that its mapping is wire-protocol semantics, not resolver policy. `TextPayload` is already a tagged enum with a single `Snapshot` variant, explicitly so a char-level variant can join without a wire break.
- `SyncService` (`crates/cartograph/src/sync/service.rs`) — identity, territory, station, pairing, device registry, and `sync_now` (which calls `vessel.register(...)` then `vessel.sync_with(station)`).
- Outpost's Track assignment is structural, in `Replica::ingest`: no parents ⇒ the Notice's own ID becomes the `TrackID`; otherwise inherit `parents[0]`'s Track. The app's only lever is what it passes as `parents` to `Vessel::author`.
- `TerritoryHold::notices_after(after)` is territory-wide, filters on `topo` only, sorted ascending. `Replica::next_topo` is an `AtomicI64` seeded from `max_topo() + 1` — a **local per-replica arrival counter**, not a global order.
- `TallyDigest::of_heads` keys on `TrackID`; `divergent()` returns the set of Tracks whose heads differ.
- Orphan parking exists at the notice-parent level only: `park_pending` / `pending_on` / `unpark`.

**What is missing or wrong**

- `Vessel::author` hardcodes `timestamp: 0` and `is_snapshot: false`.
- `EditorService` has no sync references; no code anywhere produces `payload_type: "waypoints.page.v1"` outside tests.
- `Endpoint::bind` uses `iroh::endpoint::presets::Minimal`, sets `PortmapperConfig::Disabled`, calls `.clear_ip_transports()`, and binds `127.0.0.1:0` / `[::1]:0`. `bind_routable` exists (`endpoint.rs:118`) with zero callers.
- No heartbeat, keepalive, idle timeout, or session reaping exists in `outpost-fabric` or `station.rs` (confirmed by grep).

**Constraint carried from the domain registry:** `sync` may cite `sync-protocol` guarantees, never the reverse. The fold, the arbitration key, and the Track index are all app-layer (`sync`); only HLC population, routable binds, and liveness are engine-layer (`sync-protocol`).

**Crate-layering constraint (2026-08-30):** `domain` never depends on `outpost-protocol`. The arbitration key names `NoticeID` and its derivation reads `EnvelopeHeader` — both `outpost-protocol` types — so neither the key type nor its derivation can live in `domain`; they belong with the fold in `cartograph`'s sync layer. `store`'s sync-owned persistence (winning keys, `MarkID → TrackID`, publish queue, deferred ops) names `TrackID`/`NoticeID` too; a scoped `store → outpost-protocol` dependency confined to sync-owned tables is the candidate resolution. See Open Questions.

## Goals / Non-Goals

**Goals:**

- One write path: remote ops and local ops both reach the database through `Maneuver::apply`.
- A merge law that any device can evaluate from signed content alone, with no coordination and no negotiation.
- Order-independent convergence that is _demonstrable_ — permuted delivery order is a test input, not an assumption.
- Failure containment: the Track index is an optimization whose loss degrades partitioning, never correctness.
- Enough transport to reach two real devices through a self-hosted Station, and no more.

**Non-Goals** (design-level, beyond proposal.md — Non-goals):

- No new outpost wire message types. Liveness reuses the existing session framing where possible.
- No change to outpost's Track-assignment algorithm. Per-Landmark Tracks are achieved purely by choosing `parents`.
- No attempt to make `topo` globally meaningful.
- No general-purpose op scheduler — dangling-reference deferral is a bounded, targeted mechanism.

## Decisions

### D1 (2026-07-30): Merge law is per-target last-write-wins, not a CRDT

Each `ManeuverTarget` is an independent LWW-Register. The fold sorts all ops by an arbitration key and applies them in order; the last writer to a target wins that target.

**Rationale.** An LWW-Register over a total order converges — that is the whole proof obligation, and it is small. A general CRDT would buy nothing for the case that actually motivates conflict handling here: two devices editing the _same field_ at the _same instant_, where no merge function can recover user intent and someone must lose. The cost of a CRDT (per-character metadata, tombstone growth, a much larger correctness surface) is paid on every op to improve a case that cannot be improved.

**Rejected: state-based or op-based CRDT for text.** Deferred, not foreclosed: `TextPayload` is a tagged enum, so a `Delta` variant can join later without a wire break, and `ManeuverTarget` already treats its mapping as wire semantics. If real usage shows same-paragraph concurrent editing is common, that is the moment to revisit — not before.

**Rejected: server-assigned sequence numbers.** Would make the Station a correctness dependency and break offline authoring, both contrary to local-first.

### D2 (2026-07-30): Arbitration key is `(hlc, author_verifying_key_bytes, notice_id)`

Compared lexicographically; higher wins.

**Rationale.** Every component is already in the signed `EnvelopeHeader` and AAD-bound, so it cannot be altered without invalidating the signature, and every device derives an identical key from the same bytes. `hlc` supplies causality-respecting wall-clock intent; `author_verifying_key_bytes` breaks ties between devices deterministically; `notice_id` breaks the residual tie when one device emits two ops with an identical HLC (possible only across a `logical` wrap, but the key must be total regardless). The result is a strict total order derived from content alone — no coordination, no negotiation, no local state.

**Rejected: `topo` as the ordering.** `topo` is a local arrival counter seeded per replica; two devices assign different `topo` to the same Notice. Using it would make the fold replica-dependent, which is exactly the bug this key exists to avoid. `topo` keeps its real job: a "what is new locally" cursor for incremental apply.

**Rejected: HLC alone.** Not total — two devices can produce the same packed value.

### D3 (2026-07-30): Populate `EnvelopeHeader.timestamp` from the HLC; it stays advisory in the protocol

`Vessel::author` sets `timestamp` from the Vessel's HLC instead of `0`. The field's protocol contract is unchanged: it is documented "advisory only, never ordering", `validate()` never inspects it, and it stays that way.

**Rationale.** The field is already signed and AAD-bound, which is precisely the property the arbitration key needs — an ordering input that cannot be forged in transit. Adding a parallel timestamp field would duplicate an existing signed field for no gain and would break the wire format. The layering stays honest: the protocol continues not to interpret the field; the _app_ interprets it. A Station cannot validate it and does not try.

**Consequence.** A device with a badly wrong clock can win arbitration it should lose. This is inherent to LWW and is accepted; the HLC's `receive` step bounds drift by advancing the local clock past observed remote clocks, which limits how long a lagging device stays disadvantaged.

**Rejected: a new signed ordering field in the envelope.** Wire break, and redundant with a field that already carries exactly the right guarantees.

### D4 (2026-07-30): The fold is territory-wide, never per-Track

**Rationale.** Ops legitimately span two Landmarks. Moving a Content mark from Landmark L1 to L2 is a single `UpdateTerrain { new_src: L2 }` that concerns both, but it is authored into exactly one Track. A per-Track fold would apply it in one Landmark's context and miss it in the other's — wrong even if Track assignment were perfect. Territory-wide folding has no such blind spot, and correctness here is worth more than the scan it costs.

**Mechanism.** `TerritoryHold::notices_after(after)` is already territory-wide and `topo`-ordered ascending, which makes it a sound _incremental cursor_: it yields exactly the Notices this replica has not yet folded. The fold then re-sorts that batch by the D2 key before applying — `topo` selects _what_, the arbitration key decides _order_.

**Rejected: per-Track fold for scan efficiency.** Wrong, per above. If territory-wide scans become a measured problem, the fix is indexing or incremental materialization, not narrowing the fold's input.

### D5 (2026-07-30): Persist the winning arbitration key per target

Each `ManeuverTarget` stores the key of the op currently materialized into it. On incremental apply, an op whose key is lower than the stored key is rejected without touching the row.

**Rationale.** Without this, a Notice that arrives late — after a newer one has already been folded — would clobber newer state, and the graph would depend on delivery order after all. With it, incremental apply produces the same result as a full re-fold from scratch, which is the property that makes the fold order-independent _in practice_ and not merely in a batch setting. It also gives a cheap self-check: re-folding from empty must reproduce the same graph and the same stored keys.

### D6 (2026-07-30): One Track per Landmark, via authored `parents`

Notices are authored with `parents = track_heads(track_of(landmark))`. Outpost's structural rule then keeps the Notice in that Landmark's Track.

**Rationale.** `TallyDigest::of_heads` keys on `TrackID` and `divergent()` returns differing Tracks, so Track granularity _is_ resupply granularity. One Track per Territory would mean any divergence anywhere resupplies everything. One Track per Landmark means divergence triggers resupply only for the Landmarks that actually diverged — the natural unit, because a Landmark is the natural unit of user attention and of offline editing.

Tracks are therefore a **sync-partitioning** mechanism, not a semantic boundary. Nothing about the meaning of the graph depends on Track membership; D4 already guarantees the fold ignores it.

**For ops spanning two Landmarks** (the D4 case), the Notice is authored into the Track of one of them, chosen deterministically so both devices pick the same Track for the same op. Correctness does not depend on the choice — only partitioning quality does.

**A Notice cannot belong to two Tracks, and naming both Landmarks' heads as parents does not achieve it.** `Replica::ingest` assigns exactly one Track, inherited from `parents[0]`. Authoring with `parents = [head(L1), head(L2)]` is legal, but `TerritoryHold::append_notice` clears `is_head` on _every_ parent — so L2's head stops being a head, L2 is left with no head of its own, and its subsequent history flows under L1's `TrackID`. The two Tracks fuse. `TallyDigest::of_heads` then stops listing L2 as a separate entry, which is precisely the partitioning this decision exists to preserve. Author cross-Landmark ops with a single Track's heads as parents.

**Known long-term degradation — accepted, not solved.** Because every cross-Landmark op must land in one Track or the other, repeated block movement between pages gradually connects Tracks that began separate. Over a long enough history, per-Landmark partitioning erodes toward coarser units, and resupply granularity degrades with it. This is a partitioning-quality decay, never a correctness one: the fold is Territory-wide (D4) and keys on `ManeuverTarget`, so a fused Track still folds to exactly the right graph. No mitigation is specified here deliberately — re-partitioning would mean rewriting Notice parentage, which content-addressing forbids. If the decay becomes measurable, the D11 logging is where it will show up first, and the response belongs with the snapshot layer rather than with Track assignment.

### D7 (2026-07-30): The `MarkID → TrackID` index is write-side only

The index is consulted solely to choose `parents` when authoring. It is never read during the fold.

**Rationale.** This is the change's main failure-containment decision. Because the fold never consults the index, a lost, stale, or corrupt index cannot produce a wrong graph. The worst case is that a Landmark's future Notices start a new Track, so its history fragments across two Tracks — the Landmark then resupplies as two units instead of one. Degraded partitioning, correct state. An index that were on the read path would turn a cache bug into data corruption; keeping it off that path means it can be rebuilt, or simply left wrong, without risk.

**Consequence.** The index can be regenerated at any time by folding and re-deriving Landmark→Track associations, and it needs no migration story.

### D8 (2026-07-30): Payload-level dangling references are deferred, then released

An op whose referenced `MarkID` has no folded `CreateMark` yet is set aside in a deferred set keyed by the missing `MarkID`, rather than applied or dropped. When that `MarkID` is created by a later fold step, its deferred ops are released and folded in arbitration-key order.

**Rationale.** Outpost's `park_pending`/`pending_on`/`unpark` solves the analogous problem one layer down (notice parents), but a Notice can be perfectly valid at the protocol layer and still reference an app-level entity that has not arrived — the parent DAG constrains Notice delivery, not `MarkID` reachability. Dropping such ops loses user data silently; applying them creates rows with no `CreateMark`, corrupting the graph. Deferral is the only option that preserves both.

The shape deliberately mirrors outpost's parking so the two are easy to reason about together, but it lives in the app layer because `MarkID` is meaningless to a Station.

**Note.** A reference can dangle permanently if the creating op is never authored (e.g. a bug, or an op lost before publication). Deferred ops therefore need to be observable — see the tripwire in D11 — rather than silently accumulating.

### D9 (2026-07-30): Routable binds are a source-socket fix, not a reachability fix

Stations call `bind_routable` and advertise a public address. Vessels also bind routable — but only so their outbound packets can leave the host.

**Rationale.** Stations are accept-only: `station.rs` calls `bind_identity` and runs an `accept()` loop, with no outbound `.connect(` anywhere. `NoticePush` travels Station→Vessel over the session the _Vessel_ opened via `Subscribe`. So the connection is always Vessel→Station, and **Vessels need no inbound reachability**. That is why NAT traversal, hole-punching, and relay are out of scope: nothing in this change's topology needs them.

The current loopback-only bind was a deliberate dev-ergonomics choice, not an oversight — dev builds lack a stable identifier, so wildcard binds make the macOS application firewall prompt on every recompile. The fix is therefore two-part: make the binds routable _and_ remove the reason loopback was chosen, by giving dev builds stable identifiers / signed binaries so the firewall authorizes once. Doing only the first would restore a daily annoyance and invite a revert.

**Forward compatibility.** Relay and DNS discovery are absent from `presets::Minimal` and matter only for Vessel-to-Vessel sync. V2V is a core architectural goal later, so nothing here assumes a Station is the only possible peer or bakes Station-ness into the transport layer.

### D10 (2026-07-30): Connection liveness — heartbeat, reaping, reconnect

Sessions carry a heartbeat; a Station reaps subscriptions that go quiet past an idle threshold; a Vessel detects a dead session and reconnects, resuming from its `topo` cursor.

**Rationale.** Nothing today detects or reaps a stale subscription — there is no heartbeat, keepalive, or idle timeout anywhere in `outpost-fabric` or `station.rs`. On desktop-to-desktop over a stable LAN this is invisible. On a phone it is the normal case: sleep, network change, cellular drop. The Station would otherwise hold subscriptions for Vessels that will never return, and the Vessel would sit on a socket that silently stopped delivering `NoticePush` — the failure mode where sync appears to work and no data moves.

Resuming from the `topo` cursor makes reconnect cheap and idempotent: it is the same incremental path used at startup, so reconnect is not a special case.

### D11 (2026-07-30): Measure log growth instead of solving it

Each sync logs Notice count and hold file size. A documented tripwire records the thresholds at which snapshots/compaction must be reconsidered.

**Rationale.** Unbounded log growth is real — `Vessel::author` hardcodes `is_snapshot: false`, and `snapshot_base` resolution is deferred inside outpost itself, so snapshots cannot be built at the app layer alone. Accepting the growth is right for now, but accepting it _blindly_ is how it becomes an emergency. Measurement is nearly free, requires no design commitment, and turns a future decision into a data-driven one. The same logging surfaces the D8 deferred-op count.

### D12 (2026-07-30): "History of a Mark" is a target-filtered op query, never a Track read

Any per-Mark or per-Landmark history view is derived by filtering folded operations on `ManeuverTarget`. It is never derived by reading a Track's Notices.

**Rationale.** A Track looks like the history of a Landmark and is not one, which makes this an attractive and recurring mistake. Two structural facts defeat it. First, ops legitimately span two Landmarks (D4) but land in only one Track, so a Track is already missing history that belongs to a Landmark it touched. Second, Tracks fuse over time (D6), so Track membership drifts further from Landmark identity the longer a Territory lives. A Track answers "what must be resupplied together" — a transport question. It does not answer "what happened to this thing."

Filtering on `ManeuverTarget` answers it exactly, is independent of partitioning, and needs no new storage: it is the same keying the fold already computes and the same key D5 already persists per target.

**Consequence.** `track_notices(track)` is a debugging and transport-inspection tool, not a product surface. Anything user-facing — page history, per-field change log, "what changed while I was offline" — reads the target-filtered op view.

## Risks / Trade-offs

- **Clock skew lets a wrong-clocked device win arbitration** → Inherent to LWW (D3). The HLC's `receive` step advances the local clock past observed remote clocks, bounding how long a lagging device is disadvantaged. Accepted knowingly; the alternative is coordination, which breaks offline authoring.
- **Concurrent same-field edits lose one side's text entirely** → The genuine unwinnable case (D1). Per-Landmark Tracks and the field-level targets from `op-log-completeness` shrink the blast radius from "the document" to "that one field". A char-level `TextPayload` variant can land later without a wire break.
- **Territory-wide fold scans everything** → Bounded by the incremental `notices_after` cursor (D4): the full scan happens on first fold and on deliberate re-fold, not per sync. If it becomes a measured problem, index or materialize incrementally — do not narrow the fold's input.
- **Unbounded log growth** → Explicitly accepted (D11) with measurement and a tripwire. The risk is that the tripwire is never read; the mitigation is logging it on every sync rather than behind a debug flag.
- **Track index drift fragments a Landmark's history** → Contained by D7: write-side only, so the cost is resupply granularity, never correctness. Rebuildable.
- **Tracks fuse as blocks move between Landmarks, eroding partitioning** → Accepted and unmitigated (D6). Every cross-Landmark op must land in one Track, so separate Tracks gradually connect and resupply granularity coarsens over a Territory's lifetime. Never a correctness issue — the fold is Territory-wide and keys on target. Not fixable by re-partitioning, since that would mean rewriting Notice parentage, which content addressing forbids. D11's logging is where the decay would first become visible.
- **Someone builds a history view on `track_notices`** → Forbidden by D12. The failure is subtle rather than loud: such a view is merely _incomplete_ (missing cross-Landmark ops) and degrades as Tracks fuse, so it would ship looking correct. The spec states the prohibition rather than relying on the reviewer noticing.
- **Permanently dangling references accumulate** → D8 defers rather than drops; deferred count is logged (D11) so silent accumulation becomes visible.
- **Routable binds reintroduce firewall prompts on every dev rebuild** → D9 treats stable dev identifiers as in-scope work, not a follow-up. If that half slips, the ergonomics regression will motivate reverting the transport half.
- **Publish-then-apply crash window drops a user edit** → Local apply and durable enqueue for publication are one unit; a crash leaves the op queued and it publishes on next sync. Re-publication must be idempotent, which the arbitration key already gives (same key ⇒ same outcome).
- **Depends on `op-log-completeness`** → This change cannot land first. Publishing an incomplete op log would sync a graph that silently diverges from local state, which is worse than not syncing at all.

## Migration Plan

No data migration. The only existing Territory data is transient dev seed; it is reseeded rather than migrated, and no backfill path is written. Rollback is to stop publishing — a Territory containing `waypoints.page.v1` Notices is inert to a build that does not fold them, since the fold is the only consumer.

## Open Questions

- **Where the HLC and the arbitration-key type live (raised 2026-08-30; settled 2026-09-04).**
  Resolved along the sketched lines:
  - The HLC migrated from `crates/domain/src/sync/hlc.rs` to `crates/outpost-protocol/src/hlc.rs`
    unchanged in semantics (chrono swapped for `std::time`, specta export dropped — nothing
    consumed the TS type). `domain` no longer exports `HLC`/`Clock`/`SystemClock`.
  - The arbitration-key type and its derivation live in `cartograph`'s sync layer
    (`crates/cartograph/src/sync/arbitration.rs`), reaching protocol types through the existing
    `outpost::protocol` re-export — no new cartograph dependency.
  - `store` takes the scoped `outpost-protocol` dependency when sync-owned persistence lands
    (tasks section 5), confined to sync-owned tables as sketched.

- **Hardware verification of the Success Condition is pulled from this change (2026-09-30).**
  Task 13.5 — two real devices, each with its own Vessel, against a self-hosted Station,
  including after offline divergence — cannot be performed before this change lands. It needs a
  nightly build carrying outpost, and that nightly only exists once this is merged. A dev build
  is not a substitute: mobile deployment would keep the phone tethered to the laptop, collapsing
  the multi-network condition the test exists to exercise.

  What 13.1–13.4 establish in software is unchanged — convergence, offline divergence, permuted
  delivery, cross-Landmark moves. What remains unproven is the same condition over real radios
  and real NAT. Nightlies sit at the top of the release ladder and carry the weakest stability
  guarantee, which makes a nightly the right place to find this out and the wrong place to
  assume it.

  This is a live obligation, not a descope. Archiving freezes this record, so nothing written
  here resurfaces on its own, and it is deliberately not filed as an issue — there is no merge
  or pull request it could hang off. It is carried in the working tree's uncommitted TODOS.md
  instead. Defects found once it is performed become legible the usual way, as the commits that
  fix them.
