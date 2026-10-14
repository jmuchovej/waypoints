## Context

See proposal.md — Why. What the tree holds on 2026-10-14:

- **The roster type.** `crates/outpost-fabric/src/fleet.rs`: `FleetId(VerifyingKey)`, `StationRoles { storage, orderer, blob }`, `StationEntry { peer_key: VerifyingKey, addrs: Vec<String>, roles }`, `RosterBody { fleet, revision, stations }` with `encode()` (a version byte then postcard) and `sign(&SigningKey) -> FleetConfig`; `FleetConfig::{verify, verify_against(trusted_revision), station(&VerifyingKey)}`. `ed25519-dalek` has its `serde` feature on, which renders keys as byte sequences in JSON — correct but not what an operator reads.
- **Who consumes it.** `Vessel::sync_with_fleet` and `register_with_fleet` call `check_roster` (verify, then `station(remote_key)`); `subscribe` and `submit_acl` have no `_with_fleet` form. `cartograph`'s `SyncConfig` is `{ device_index, station: Option<EndpointAddr>, territory }`; `PairingPayload::new(mnemonic, territory, station)` carries the address. No roster is constructed anywhere outside tests.
- **Station and fleet key.** `Station` holds `fleet_key: Option<SigningKey>` for ordering and nothing about the Fleet otherwise; its holds are opened with `Replica::reopen` and bootstrapped with `Replica::bootstrap`, both unfleeted, so a storage Station accepts any ordered record's proof without checking it. `Replica::bootstrap_with_fleet` exists for Vessels.
- **The wire.** `SyncMessage` is a postcard-encoded enum behind ALPN `outpost/sync/v1`; the fabric's docs say wire-version negotiation happens inside the session, and no variant is marked `#[non_exhaustive]`. A new variant is a new discriminant.
- **Keys and ids.** Station and fleet keys are 32-byte seed files (`outpost keygen`); a Station's peer identity is the public half, equal to its iroh endpoint id, rendered as hex in `station.addr`.
- **Where addresses come from.** `Endpoint::advertised_addr` builds `EndpointAddr::from_parts(id, sockets)`; a roster entry's sockets plus its peer key are exactly those parts.

## Goals / Non-Goals

**Goals:**

- One roster, three readers: the operator's file, the Station's memory, the Vessel's configuration, all verifying the same bytes.
- A device can go from a pairing payload to a verified Fleet without a second out-of-band secret.
- No behaviour change for a Station or Vessel that has no roster.

**Non-Goals:**

- Beyond the proposal's: no roster caching across app restarts beyond the configuration file; no roster for the blob role.

## Decisions

### D1 — The file is a presentation; the signature covers the body's canonical encoding

**Chosen** (2026-10-14). A `RosterFile` in `fleet.rs` mirrors `FleetConfig` with explicit serde: `fleet`, `peer`, and `signature` as lowercase hex strings, `addrs` as `ip:port` strings, `roles` as a list of role names, plus a `version: 1`. `RosterFile::from(&FleetConfig)` and `TryFrom<RosterFile> for FleetConfig` convert; the signature is always over `RosterBody::encode()`, never over the JSON. `FleetConfig` keeps its derived serde for the wire. `StationEntry.addrs` changes from `Vec<String>` to `Vec<SocketAddr>` (`SocketAddr` is serde-able); the "opaque hints" comment was the placeholder this change replaces, and the roster has no other consumer to break. The relay change, when it comes, adds a second address kind beside sockets rather than re-widening to strings.

**Why JSON with hex.** Operators read and diff it, `station.addr` already renders the same identity in hex, and the Kubernetes ConfigMap that carries it is text. **Why not sign the JSON.** Canonical JSON is a project of its own; postcard over a fixed struct is already canonical and already what Vessels verify.

### D2 — `outpost fleet` is a subcommand group; `station id` sits beside `station`

`outpost fleet id --key-file F` prints the Fleet identity. `outpost station id --key-file S` prints the peer identity (and the full `EndpointAddr` with `--advertise` sockets, so an operator can build an entry from one command). `outpost fleet roster sign --fleet-key-file F --revision N --station <peer>=<socket>[,<socket>][:roles] … --out roster.json [--supersedes old.json]` composes, signs, writes; with `--supersedes` the revision must exceed the old file's. `outpost fleet roster show roster.json` verifies and prints. `keygen` stays the one key writer: a fleet key and a station key are the same bytes.

**Rejected.** _`fleet init` creating a key and a first roster in one step_ — hides the revision and the entries; two commands are clearer and the kind recipe can chain them.

### D3 — A Station with a roster is a fleeted, role-checked Station

`Station::with_roster(FleetConfig)` is a builder step after construction, but the checks need the key before binding, so the CLI verifies the file, checks `station(&own_key)` and the orderer role against `--fleet-key-file`, and only then constructs. Fleeting: `build` receives `Option<VerifyingKey>` (the roster's fleet identity) and opens holds with `Replica::reopen_with_fleet` / `bootstrap_with_fleet`; `reopen_with_fleet` is a new sibling (`reopen_inner` already takes the option). Existing holds become fleeted on the next start, which is safe: records already in the log were accepted before, and verification applies to what arrives.

### D4 — `RosterRequest` is answered without a handshake, before `Hello` or after

The dispatch arm matches `RosterRequest` whether or not a `Hello` preceded it and answers `RosterResponse { roster: Option<FleetConfig> }`, then the session ends. No admission: the roster is signed, public, and useless to forge. **Wire compatibility:** an older Station fails to decode the request and drops the session; an older Vessel never sends it. Recorded as acceptable pre-1.0; the ALPN stays `outpost/sync/v1` because every existing message is unchanged.

### D5 — The Vessel gets `_with_fleet` on every dial, and the app always uses them when a Fleet is configured

`subscribe_with_fleet` and `submit_acl_with_fleet` join the two existing ones, each calling `check_roster` after connect. `Vessel::fetch_roster(addr) -> Option<FleetConfig>` sends the request. In `cartograph`, `SyncConfig` gains `fleet: Option<FleetTrust { id: FleetId, roster: Option<FleetConfig>, trusted_revision: u64 }>`; a `trusted_roster()` helper fetches when `roster` is `None`, verifies against `id`, and persists; every service method that dials picks the `_with_fleet` form when `fleet` is `Some`. Refresh: after any successful sync, if the Station's roster revision (fetched opportunistically once per session start) exceeds `trusted_revision` and verifies, it is adopted. A lower revision is logged and ignored.

**Why the identity, not the roster, in the pairing payload.** 32 bytes fit a QR code and a URL; a roster grows with the Fleet. The joining device fetches the roster from the Station the payload names and verifies it against the identity, so a Station that is not in the Fleet cannot pass itself off: it cannot produce a roster that verifies and lists it.

### D6 — The roster in Kubernetes is a ConfigMap, optional in the base

`deploy/kubernetes/base` gains an optional `roster.json` ConfigMap mounted read-only at `/config/roster.json` with `OUTPOST_ROSTER` pointing at it; the base ships with the mount but no roster, so the one-Station Fleet keeps working unchanged. The kind recipe `kind-apply` gains the steps to derive the Station identity from the generated key and sign a one-entry roster, so the kind run exercises the fleeted path.

## Risks / Trade-offs

- [A new wire message breaks old-peer decoding] → Pre-1.0; the only deployed peers are the audit ones. The design notes it so that the first post-1.0 wire change brings versioning with it.
- [Changing `StationEntry.addrs` to `SocketAddr` breaks the two tests that build rosters with `vec![]` / string hints] → Updated with the change; no other consumer.
- [Fleeting existing holds on the next start could refuse records a Station previously accepted] → Verification applies to incoming records only; records already in a hold are not re-verified. A hold that contains a bad proof from an unfleeted past stays as it is, which is the status quo.
- [The app persists a roster in its configuration file, which grows with the Fleet] → Tens of entries at most; a JSON file.
- [Pairing payloads from older app versions lack the identity] → The field is optional; the single-Station path remains.

## Migration Plan

1. Land the file format, tooling, and `station id` with round-trip tests.
2. Land the Station side: `--roster`, start-up checks, fleeted holds, the roster request.
3. Land the Vessel and app side: `_with_fleet` on every dial, `fetch_roster`, the configuration, the pairing field.
4. Land the manifests and the kind recipe.
5. Rollback is per step; a Station without `--roster` and an app without a Fleet are unchanged throughout.

## Open Questions

- Whether `outpost fleet roster show` should also print the entries as `station.addr`-style JSON per Station, for pasting into the app. Does not affect the format.
