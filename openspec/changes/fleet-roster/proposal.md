## Why

A Fleet's trust basis exists in code and nowhere else. `outpost-fabric` defines the signed, revision-numbered roster (`FleetConfig`), the Vessel can verify a Station against one (`register_with_fleet`, `sync_with_fleet`), and the roster-peer admission arm is promised in a doc comment — but nothing produces a roster, no Station holds one, no Vessel is ever handed one, and the app pins a single Station address with no notion of which Fleet it belongs to. `deployable-station` deliberately left this out: a roster nothing reads would have been a file for its own sake.

Everything beyond one Station needs it. Station-to-Station replication admits peers by roster; a Vessel that fails over needs a roster to fail over within; the hosted tier's operators need a way to mint a Fleet and sign its membership. This change makes the roster real end to end: produced by a tool, held and served by Stations, verified by Vessels, with the Fleet identity travelling in the pairing payload so a device joining a Territory knows which Fleet to trust before it trusts any Station.

## What Changes

- **A roster file format and the tooling that produces it.** `outpost fleet id` prints a fleet key's Fleet identity; `outpost station id` prints a Station key's peer identity; `outpost fleet roster sign` composes a roster from Station entries (peer identity, dialable sockets, roles) and a revision, signs it with the fleet key, and writes it; `outpost fleet roster show` verifies and prints one. The file is JSON with hex identities, a presentation of the body the fleet key signs; the signed bytes stay the existing canonical encoding, so the wire and the file agree.
- **A Station holds its roster.** `--roster` (`OUTPOST_ROSTER`) loads a roster file at start-up; the Station refuses to start if its own key is not listed, and refuses the orderer role if the roster marks it a storage-only Station. A Station with a roster homes its Territories _fleeted_: it verifies order proofs against the roster's fleet identity, which no Station does today.
- **A Station serves its roster.** A new request answers with the Station's current roster to any caller, no handshake required: the roster is a signed public document, and the Station is where a Vessel with only an address can fetch it. A Station without a roster answers that it has none.
- **A Vessel homed to a Fleet verifies every Station it dials.** The app's sync configuration gains the Fleet identity and the roster it trusts; with one present, registration, sync, subscription, and ordering refuse a Station the roster does not list, and a roster with a lower revision than the trusted one is rejected. The roster is fetched from the configured Station and refreshed when a newer revision is served.
- **Pairing carries the Fleet identity.** The payload a new device scans gains the Fleet identity beside the Station address; the joining device fetches the roster from that Station, verifies it against the identity, and only then registers. A payload without a Fleet identity keeps today's single-Station behaviour.
- **`outpost sync` accepts `--roster`.**

## Capabilities

**`sync-protocol`** carries the roster format, the Station's roster behaviour, and the roster request. **`sync`** carries the app's side: the trusted Fleet in the sync configuration, verification before every dial, and the pairing payload. `sync` may cite `sync-protocol`'s guarantees and does.

### New Capabilities

None.

### Modified Capabilities

- `sync-protocol`: new requirements _A Fleet roster is a signed file operators produce_, _A Station holds and serves its roster_, and _A Station with a roster homes Territories fleeted_.
- `sync`: new requirements _A Vessel homed to a Fleet trusts only rostered Stations_ and _Pairing names the Fleet_.

## Non-goals

- **Station-to-Station replication and the roster-peer admission arm.** The next change; it needs this one.
- **Multiple Stations in the app's configuration, or choosing among them.** The app keeps one configured Station; the roster is what it verifies that Station against. Failover and sub-roster selection are the change after replication.
- **Relay addresses in roster entries.** Entries hold IP sockets; the relay change extends the entry format.
- **Roster distribution beyond the Station itself** (a well-known URL, a QR code of the roster). The pairing payload carries the identity, the Station serves the roster; other transports can be added without changing either.
- **Rotating a fleet key.** A new fleet key is a new Fleet.

## Impact

- `crates/outpost-fabric/src/fleet.rs` — the file representation with hex identities and `ip:port` address strings; conversion to and from `FleetConfig`; `StationEntry` addresses become typed sockets rather than opaque strings (the comment calling them opaque was a placeholder).
- `crates/outpost-protocol/src/sync.rs` — `RosterRequest` and `RosterResponse { roster: Option<…> }`. A wire addition: an older peer cannot decode the new discriminant, which is acceptable before 1.0 and recorded in the design.
- `crates/outpost/src/station.rs` — `with_roster`, the start-up checks, fleeted holds (`Replica::bootstrap_with_fleet` / `reopen` with the fleet key), the roster request arm.
- `crates/outpost/src/vessel.rs` — `fetch_roster`, and the `_with_fleet` paths gaining `subscribe_with_fleet` and `submit_acl_with_fleet`.
- `crates/outpost/src/bin/outpost/commands/{fleet.rs,station.rs,sync.rs}` — the `fleet` subcommand group, `station id`, `--roster`.
- `crates/cartograph/src/sync/{service.rs,types.rs}` and the pairing payload — the trusted Fleet in `SyncConfig`, verification on every dial, roster refresh, the payload field; TypeScript bindings regenerate.
- `crates/outpost/tests/` — roster round-trip, Station start-up refusals, the roster request, fleeted holds rejecting a forged proof over sync, and a Vessel refusing an unrostered Station on every path.
- `deploy/kubernetes/base` — a `--roster` mount from a ConfigMap, optional in the one-Station base.
