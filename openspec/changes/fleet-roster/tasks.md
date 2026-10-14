## 1. Roster file and tooling

- [ ] 1.1 Change `StationEntry.addrs` to `Vec<SocketAddr>` in `crates/outpost-fabric/src/fleet.rs` and add `RosterFile` (hex identities and signature, `ip:port` sockets, role names, `version: 1`) with conversions both ways and `read_file`/`write_file`; verify a unit test that a signed roster round-trips through its file and still verifies, and that editing one byte of the file fails verification naming the file
- [ ] 1.2 Add `crates/outpost/src/bin/outpost/commands/fleet.rs` with `fleet id`, `fleet roster sign` (`--fleet-key-file`, `--revision`, repeatable `--station <peer>=<socket>[,<socket>][:roles]`, `--out`, `--supersedes`), and `fleet roster show`; register the group in `main.rs`; verify a process test signs a two-Station roster, `show` verifies it, and signing with `--supersedes` at an equal revision exits non-zero naming both revisions
- [ ] 1.3 Add `outpost station id --key-file` printing the peer identity (and the `EndpointAddr` JSON with `--advertise`); verify a process test that the printed identity equals the `id` in the address a Station started on that key logs

## 2. The Station's roster

- [ ] 2.1 Add `SyncMessage::{RosterRequest, RosterResponse { roster }}` to `crates/outpost-protocol/src/sync.rs` with a note on wire compatibility; verify the protocol crate's wire round-trip tests cover both
- [ ] 2.2 Add `Replica::reopen_with_fleet` beside `reopen` (both over `reopen_inner`); verify a unit test that a fleeted reopened replica refuses a forged proof and accepts a genuine one
- [ ] 2.3 Thread an optional Fleet identity through `Station::build` so holds are opened and bootstrapped fleeted when a roster is held, and add `Station::with_roster(FleetConfig)` plus the `RosterRequest` dispatch arm answering before or after `Hello`; verify integration tests: a rostered Station refuses a forged proof over sync, serves its roster to a handshake-less caller, and an unrostered Station answers `None`
- [ ] 2.4 Add `--roster` (`OUTPOST_ROSTER`) to `commands/station.rs`: verify the file, refuse to start when the Station's key is absent or when `--fleet-key-file` is given without the orderer role, then construct; verify process tests for both refusals (exit non-zero, message names the file or the role) and for a rostered start that logs the roster revision

## 3. The Vessel and the app

- [ ] 3.1 Add `Vessel::fetch_roster`, `subscribe_with_fleet`, and `submit_acl_with_fleet` in `crates/outpost/src/vessel.rs`; verify integration tests that each `_with_fleet` path refuses an unrostered Station and that `fetch_roster` returns the Station's roster
- [ ] 3.2 Add `FleetTrust { id, roster, trusted_revision }` to `SyncConfig` in `crates/cartograph/src/sync/service.rs`, a `trusted_roster()` that fetches and verifies on first use, the `_with_fleet` selection on every dial, the opportunistic refresh with rollback rejection, and `station_set` accepting a Fleet identity; regenerate the TypeScript bindings; verify cartograph tests: unrostered Station refused on each path, first-use fetch persists the roster, a higher revision is adopted, a lower one is kept out
- [ ] 3.3 Add the optional Fleet identity to `PairingPayload` and make the joining path fetch and verify the roster before registering; verify a cartograph test where a payload names a Fleet and a Station not in that Fleet's roster is refused, and one where it is listed succeeds
- [ ] 3.4 Add `--roster` to `outpost sync`; verify it refuses an unrostered Station and syncs with a rostered one

## 4. Deployment and close-out

- [ ] 4.1 Add the optional roster ConfigMap mount and `OUTPOST_ROSTER` to `deploy/kubernetes/base`, and extend `deploy/justfile`'s `kind-apply` to derive the Station identity and sign a one-entry roster for the kind overlay; verify `just deploy::lint` passes and a kind run logs the roster revision and serves it to the audit harness
- [ ] 4.2 Add `register_and_sync_with_fleet` to `tests/audit.rs` driven by `OUTPOST_AUDIT_ROSTER`; verify it passes against the kind pod and fails against a Station started without the roster
- [ ] 4.3 Update the sync section of `.agents/skills/waypoints-architecture/references/architecture.md` for the roster, the fleeted Station, and the pairing field; verify it names the file format and the request
- [ ] 4.4 Run `jj fix -s @`, the hook steps, and `just check-generated` (the bindings); verify all green
