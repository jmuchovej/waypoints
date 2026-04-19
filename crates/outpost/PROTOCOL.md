# Waypoints Sync Protocol — v1

How Waypoints installations exchange edits. This document is the
specification; the `outpost` binary in this crate is the reference
implementation of it.

## 1. Topology: peers, not servers

Every Waypoints installation is a **peer**. A peer holds an append-only
envelope store (the app's `sync_log`) and could, in principle, answer the
two requests this protocol defines. What we informally call a *relay* is
just a peer with two special properties:

1. **Always online** — it exists so that two devices that are never awake
   at the same time can still converge.
2. **Never originates edits** — it stores and serves envelopes verbatim
   and interprets nothing inside them.

This is deliberately the Syncthing shape. Anyone can stand up a relay
(self-hosting is a design goal, even where v1 tooling doesn't ship it),
and a future app version can serve these endpoints itself for LAN /
device-to-device sync without any protocol change.

The relay is **replaceable and dumb** on purpose: conflict resolution,
schema knowledge, and authority over data live exclusively in the app's
Rust core. A relay that tried to be clever would be wrong.

### Terminology: not Syncthing's "relay"

We borrow Syncthing's *shape*, not its vocabulary — Syncthing uses the
word "relay" for a different component. The mapping:

| Syncthing component | What it does | Waypoints analogue |
| ------------------- | ------------ | ------------------ |
| Peer (a Syncthing instance) | holds data, reconciles indexes with other peers | a Waypoints installation |
| Untrusted always-on device (e.g. a NAS node) | stores data it can't read, so peers that are never simultaneously online still converge | **what we call a relay** |
| Relay (`strelaysrv`) | pipes encrypted bytes between two *live* peers when NAT traversal fails; stores nothing | none — a possible future transport concern (§8) |
| Discovery server | maps device IDs to current addresses; holds no data | none — peers are configured statically |

A Waypoints relay is Syncthing's *always-on peer*, not its relay:
store-and-forward, never live-forwarding. If live forwarding or peer
discovery infrastructure ever arrives, those are new components beside
this one, not versions of it.

### Gossip: relays replicating from relays

A relay may also *pull*: run the §4.2 fetch loop against another relay
and append what it receives to its own store, exactly as if a client
had uploaded it. This is classical pull-based anti-entropy, and it is
what turns isolated hub-spoke stars into a converging mesh. Nothing
about it changes the protocol — a gossiping relay is just a peer
exercising the client half — and it preserves the relay's defining
property: it carries envelopes, it never originates or interprets them.

Obligations of a pulling relay (a subset of §5 — it stores, never
applies):

1. **Realm-guard** each received envelope (§5.2).
2. **Append idempotently** — `group_id` uniqueness already does this.
   Idempotency is also what terminates propagation around cycles: A
   pulls from B while B pulls from A, and every envelope settles after
   one lap.
3. **Assign fresh seqs.** Gossiped envelopes get this relay's own
   arrival seqs; the same envelope has different seqs on different
   relays, which §3 already made safe.
4. **Track per-(peer, realm) cursors**, advanced only after the local
   append succeeds.
5. **Never rewrite payloads** — byte-verbatim in, byte-verbatim out.

Drift quarantine (§5.3) is deliberately *not* on this list: it protects
a peer's local HLC clock, and a relay has none. Quarantine happens at
apply time, on devices.

Gossip is pull-only by design. The fetch path needs only a realm grant,
whereas upload in official mode also checks that each envelope's
`account_id` matches the token's identity (§4.1) — a constraint a
carrier hauling other accounts' envelopes cannot meet. Convergence
therefore requires each edge of the peering graph to be configured on
the *pulling* side: if A should hold B's envelopes, A must list B as a
peer. Propagation latency is poll interval × graph diameter.

### Identity scoping

| ID          | Scope of uniqueness                    | Guaranteed by            |
| ----------- | -------------------------------------- | ------------------------ |
| `AccountID` | global                                 | the accounts database    |
| `RealmID`   | global (independent of account)        | UUIDv4 collision odds    |
| `DeviceID`  | per install                            | UUIDv4, minted locally   |
| `group_id`  | global (envelope idempotency key)      | UUIDv4, minted per group |
| everything else (`MarkID`, `TerrainID`, `FeatureID`, …) | scoped to its territory/realm | UUIDv4 |

The **accounts database** (Turso, operated by Waypoints, not
self-hostable) exists to anchor account identity and device-token
authentication. It is *not* a participant in sync: no envelope ever
flows through it, and self-hosted relays must not need it (see §6).

## 2. The envelope

The unit of exchange is the `Envelope` — one command group, wrapped with
the identity and clock context needed to apply it anywhere:

```json
{
  "group_id":   "9c1f<uuid>",
  "account_id": "9c1f<uuid>",
  "realm_id":   "9c1f<uuid>",
  "device_id":  "9c1f<uuid>",
  "hlc":        "113817913650446336",
  "checkpoints": [
    { "op": "create_mark",    "mark_id": "…", "data": { } },
    { "op": "update_rich_text", "mark_id": "…",
      "new": { "kind": "Snapshot", "rich_text": { } } },
    { "op": "create_terrain", "edge_id": "…", "src": "…", "dst": "…",
      "kind": "child", "sequence": "i000", "extra": null }
  ],
  "touched_marks": ["9c1f<uuid>"]
}
```

- `hlc` is a hybrid logical clock packed into a single i64 —
  48 bits of milliseconds since the UNIX epoch, 16 bits of logical
  counter — and carried **as a decimal string**: packed values exceed
  `Number.MAX_SAFE_INTEGER`, so a bare JSON number would be silently
  corrupted by any JavaScript participant that re-parses the envelope.
  (The top physical bit doubles as the i64 sign bit, so the usable range
  is 47 bits — fine until the year ~6429.) Relays that index by HLC
  parse it as `BigInt`/i64; `seq` values, by contrast, are small and
  stay plain numbers.
- `checkpoints` is the ordered list of maneuvers (tagged with `"op"`,
  snake_case). Relays treat the array — and everything inside it — as an
  opaque blob. Maneuver fields marked undo-only in the app
  (`old`, `old_src`, `old_order`) never appear on the wire.
- `device_id` is the originating device's real ID at origination time.
  It is never rewritten (device aliasing is a local UI concept), because
  it participates in conflict resolution.
- `touched_marks` is populated but unread in v1 (reserved for dependency
  checks).
- There is no top-level version field in v1. If v2 needs one, it will be
  `protocol_version` with `#[serde(default)] = 1` semantics; unknown
  *maneuver* or *payload* variants must cause the receiving peer to
  reject the whole envelope rather than partially apply it.

## 3. Two orderings, deliberately

The protocol keeps two orders that must never be conflated:

1. **HLC order** decides conflicts. Every row a maneuver touches carries
   the `(hlc, device_id)` of its last winning write; last-write-wins
   compares against that key. HLC order is global, causal-ish, and set
   at *origination* time.
2. **Arrival order (`seq`)** decides delivery. Each relay assigns every
   stored envelope a strictly increasing integer `seq` **local to that
   relay** (per realm), in the order envelopes *arrived there*.

Why delivery can't cursor on HLC: a device that was offline for a week
uploads envelopes whose HLCs are a week old. Any peer whose HLC-cursor
already passed that week would never fetch them — silent, permanent data
loss for exactly the device sync exists to serve. Arrival order has no
such gap: late uploads get fresh seqs.

Consequences for clients:

- **Cursors are per-source.** `seq` values from different relays are
  unrelated. A client tracks `(relay, last_seq)` for every relay it
  talks to (`relay_cursors` locally).
- **Overlapping sources are safe.** Envelope application is idempotent
  on `group_id`; fetching the same envelope from two relays applies it
  once. This is what makes multi-relay (and eventual peer-to-peer)
  topologies safe without coordination.
- **Order independence.** LWW resolution makes the *semantic* outcome
  independent of application order; seq order matters only for cursor
  bookkeeping.

## 4. Endpoints

Two endpoints, realm-addressed. All bodies are JSON. All requests carry
`Authorization: Bearer <device-token>` (§6).

### 4.1 Upload

```
POST /realms/{realm_id}/envelopes
Content-Type: application/json

{ "envelopes": [ Envelope, … ] }
```

Response `200`:

```json
{
  "results": [ { "group_id": "…", "seq": 4132 }, … ],
  "max_seq": 4133
}
```

- **Idempotent.** `group_id` is unique per relay; re-uploading an
  already-stored envelope returns its existing `seq` (`INSERT … ON
  CONFLICT DO NOTHING` + read-back). Clients retry freely.
- The relay validates only: token scope (does this device belong to an
  account allowed to write this realm), `realm_id` in the path matches
  each envelope, that each envelope's `account_id` matches the token's
  identity (when the auth mode carries one — shared-secret mode has no
  per-device identity and skips this, see §6), and a payload size cap.
  It does not parse `checkpoints`.
- Batch cap: 100 envelopes or 1 MiB per request, whichever is hit
  first (`413` beyond it).

### 4.2 Fetch

```
GET /realms/{realm_id}/envelopes?since_seq=N&exclude_device=D&limit=L
```

Response `200`:

```json
{
  "envelopes": [ { "seq": 4130, "envelope": Envelope }, … ],
  "next_since": 4133,
  "has_more": false
}
```

- Returns envelopes with `seq > since_seq`, ascending, at most `limit`
  (default 200, max 500).
- `exclude_device` filters out the caller's own envelopes (a bandwidth
  optimization only — clients must tolerate receiving their own
  envelopes and drop them via idempotency).
- `next_since` is the cursor to persist and pass next time; it advances
  even past envelopes the client chooses to quarantine (see §5).
- `has_more: true` means call again immediately with `next_since`.

That's the whole protocol. A conforming relay is ~200 lines against any
SQLite-compatible store.

## 5. Client obligations

A peer consuming a relay must:

1. **Apply idempotently** — skip envelopes whose `group_id` is already
   in its local log.
2. **Guard the realm** — drop envelopes whose `realm_id` isn't the one
   it asked for (defense against a misconfigured relay).
3. **Quarantine excessive drift** — reject envelopes whose HLC physical
   component is more than `MAX_HLC_DRIFT_MS` (15 minutes) ahead of the
   local wall clock, *before* letting the HLC touch the local clock.
   A broken-clock device must not poison the realm. The cursor still
   advances past quarantined envelopes: the divergence is deliberate and
   attributable to the broken device.
4. **Track cursors per relay** and derive nothing about one relay's seq
   from another's.
5. **Upload its outbox** — every locally-originated envelope not yet
   acknowledged by the relay, in local order. Uploads are at-least-once;
   the relay's idempotency makes the effect exactly-once.

## 6. Authentication

**Official relay (v1):** requests authenticate with a per-device token
minted at device registration and stored in the accounts database
(`device_tokens`: token hash → `account_id`, `device_id`, `revoked_at`).
Outpost validates the token and checks the account may access the
realm. This is the only place the accounts database appears in the sync
path — as an *authenticator*, never as a store.

**Self-hosted relays:** must not depend on the global accounts
database. Outpost's validator is a single pluggable seam; self-hosted
instances run `OUTPOST_SHARED_SECRET` mode (realm-membership proofs are
a possible future refinement). Protocol unchanged.

## 7. Reference storage schema

What outpost stores in its embedded turso/SQLite file (any SQLite
works — this is `schema.sql` in this crate):

```sql
CREATE TABLE IF NOT EXISTS envelopes (
  seq         INTEGER PRIMARY KEY AUTOINCREMENT, -- arrival order: THE cursor
  group_id    TEXT NOT NULL UNIQUE,              -- idempotency key (uuid)
  account_id  TEXT NOT NULL,
  realm_id    TEXT NOT NULL,
  device_id   TEXT NOT NULL,
  hlc         INTEGER NOT NULL,
  payload     TEXT NOT NULL,                     -- Envelope JSON, verbatim
  received_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_envelopes_realm_seq   ON envelopes (realm_id, seq);
CREATE INDEX IF NOT EXISTS idx_envelopes_realm_device ON envelopes (realm_id, device_id, seq);
```

Tenancy note: the official relay keeps all realms in one database,
scoped by `realm_id`. Because the protocol is realm-addressed and `seq`
is per-relay anyway, moving a realm (or account) to a dedicated database
later is invisible to clients.

## 8. Non-goals in v1 (and their shapes when they come)

- **Checkpoints / log compaction.** New devices replay the realm from
  `seq 0`; the log grows unbounded. The fix — a materialized snapshot at
  seq N plus the tail — is the *same mechanism* as backfilling pre-sync
  data and GC'ing the log. One feature, three birds, deliberately
  deferred until log size is measurable.
- **Push.** v1 polls (~5s). Push arrives as a *nudge* ("you have mail"
  via Durable Object / WebSocket / SSE), never as data delivery, so the
  cursor path stays the single source of truth.
- **End-to-end encryption.** `payload` is already opaque to the relay;
  E2E is "the blob becomes ciphertext" plus key distribution. Wire shape
  survives.
- **In-app serving / LAN sync.** The app answering §4 from its own
  `sync_log`. Needs listeners, discovery, NAT thinking — protocol
  already accommodates it (per-source cursors), implementation does not.
  The decided shape: the sync engine lives in this crate behind cargo
  features (the app pulls in the client/`EnvelopeStore` surface without
  the server or storage backends), and the app serves from `sync_log`
  once that table gains an arrival-order `local_seq` and a byte-verbatim
  `payload` column — verbatim so a peer can carry envelopes it cannot
  parse (a newer app version's, or later, ciphertext).
- **Per-peer upload tracking.** v1 tracks "uploaded to *the* relay" as a
  single flag. Multiple upload targets want an explicit `local_seq` on
  the log and per-peer high-water marks (Syncthing-style) — the same
  `local_seq` in-app serving needs. Schema note, not a v1 need.
