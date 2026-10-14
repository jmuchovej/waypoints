## MODIFIED Requirements

### Requirement: A Station is reachable at an advertised address

A Station SHALL bind a routable socket and SHALL advertise an address at which Vessels can reach it. A Station SHALL accept sessions from Vessels and SHALL NOT initiate connections to Vessels. The only connections a Station initiates are to Stations listed in its roster, for replication; a Station without a roster initiates none.

A Station SHALL bind its UDP port on every address family it serves and SHALL refuse to start if the port cannot be bound; it SHALL NOT fall back to a different port. The port is 6782 unless the operator configures another. A configured port of 0 requests an ephemeral port, the posture for same-host development and tests.

At startup the Station SHALL publish the address it advertises where an operator can read it without shell access to the data directory: in its log output as well as in the address file.

#### Scenario: A Vessel connects to a self-hosted Station

- **WHEN** a Vessel on a different host is configured with a Station's advertised address
- **THEN** it establishes a session with that Station

#### Scenario: Live delivery uses the Vessel's session

- **WHEN** a Station has a Notice to deliver to a subscribed Vessel
- **THEN** it is delivered over the session the Vessel opened
- **AND** the Station does not attempt to connect to the Vessel

#### Scenario: Only roster peers are dialed

- **WHEN** a Station with a roster runs a replication cycle while a Vessel that is not in the roster is listening on a reachable socket
- **THEN** the Vessel receives no connection, and every connection the Station initiates is to an address from its roster

#### Scenario: The default port serves

- **WHEN** a Station is started with no port configured
- **THEN** it serves on UDP port 6782

#### Scenario: Port zero is ephemeral

- **WHEN** a Station is started with port 0 configured
- **THEN** it serves QUIC on a port the system assigned, and its published address names that port
- **AND** its probe routes answer over TCP at that same port

#### Scenario: A configured port is the port that serves

- **WHEN** a Station is started with a port configured and a Vessel dials the Station's advertised address at that port
- **THEN** the session is established on that port

#### Scenario: A configured port is unavailable

- **WHEN** a Station is started with a port configured and another process already holds it
- **THEN** the Station exits with a non-zero status and an error naming the port
- **AND** it does not serve on any other port

#### Scenario: The advertised address is visible to the operator

- **WHEN** a Station starts
- **THEN** its log output contains the complete advertised address a Vessel can be given, identical to the address file's content

## ADDED Requirements

### Requirement: Stations in a Fleet replicate every Territory

A Station started with a roster SHALL replicate from every storage Station in that roster other than itself: once its stored Territories are loaded, on a configurable interval thereafter, and promptly after it homes a new Territory. For each Territory a peer serves, the Station SHALL reconcile the Territory if it already holds it, and otherwise SHALL obtain the peer's ordered ACL log, bootstrap a fleeted hold from it, and then reconcile. Replication SHALL move only what sync moves — ordered ACL records and Notices as ciphertext and envelopes — and SHALL verify order proofs against the roster's Fleet identity. A peer that is unreachable SHALL be skipped for that cycle and retried on the next; a Station SHALL NOT fail or stop serving because a peer is down.

#### Scenario: Two Stations converge

- **WHEN** an owner registers a Territory and syncs a Notice with one of two rostered Stations
- **THEN** within one replication interval the other Station serves the Territory and holds the Notice

#### Scenario: A new Station backfills Territories whose owners are offline

- **WHEN** a Station joins a Fleet whose peers hold Territories, and no Vessel syncs with it
- **THEN** after its first cycle it serves every Territory its peers serve, with their Notices

#### Scenario: A registration spreads without waiting for the interval

- **WHEN** a Territory is homed on one Station
- **THEN** the other Stations hold it well before the next scheduled cycle

#### Scenario: A down peer is skipped

- **WHEN** one rostered Station is unreachable during a cycle
- **THEN** the other peers are still replicated from, the Station keeps serving, and the next cycle retries the down peer

#### Scenario: Replication carries no plaintext

- **WHEN** a Notice replicates between Stations
- **THEN** the receiving Station holds exactly the ciphertext and envelope the sending Station held, and neither Station can decrypt it

### Requirement: Roster peers are admitted by identity

A Station with a roster SHALL admit a caller whose authenticated identity is a storage Station in that roster without a handshake: to open and reconcile any Territory it serves, to request the list of Territories it serves, and to request the ordered ACL log of one. These two requests SHALL be answered only to roster peers; to any other caller, and on a Station without a roster, they SHALL be refused.

#### Scenario: A roster peer syncs without a handshake

- **WHEN** a rostered Station opens a Territory on another rostered Station without sending a handshake
- **THEN** the session is admitted and both the ACL and the Notice phases are served

#### Scenario: A Vessel cannot list Territories

- **WHEN** a Vessel, authenticated or not, sends the Territory list request
- **THEN** it is refused

#### Scenario: An unrostered Station is a stranger

- **WHEN** a Station whose identity is not in the roster opens a Territory without a handshake
- **THEN** it is refused exactly as an anonymous caller is under the Members policy
