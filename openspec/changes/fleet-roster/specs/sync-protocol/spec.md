## ADDED Requirements

### Requirement: A Fleet roster is a signed file commodores produce

A Fleet roster SHALL be a file holding the Fleet identity, a monotonic revision, and one entry per Station — the Station's peer identity, the sockets at which it is dialable, and its roles — together with the fleet key's signature over the roster body. The file SHALL render identities as lowercase hex and sockets as `ip:port` strings; the signature SHALL cover the canonical encoding of the body, so that a roster reconstructed from the file verifies identically to one received over the wire. Commodore tooling SHALL derive a Fleet identity from a fleet key, a peer identity from a Station key, compose and sign a roster from entries and a revision, and verify and display an existing roster. The tooling SHALL refuse to sign a roster whose revision is not greater than the revision of an existing roster it is given to supersede.

#### Scenario: A roster round-trips through its file

- **WHEN** a roster is signed to a file and read back
- **THEN** the reconstructed roster verifies against the Fleet identity in the file, and lists the same Stations, sockets, roles, and revision

#### Scenario: A tampered file does not verify

- **WHEN** any field of a roster file is edited after signing
- **THEN** reading it reports a verification failure naming the file

#### Scenario: Revisions only go up

- **WHEN** a commodore signs a new roster with a revision no greater than the one it supersedes
- **THEN** the tooling refuses and names both revisions

#### Scenario: Identities are derivable from keys

- **WHEN** a commodore derives the peer identity from a Station's key file
- **THEN** it equals the endpoint identity that Station publishes in its address when started on that key

### Requirement: A Station holds and serves its roster

A Station MAY be started with a roster. A Station given a roster SHALL verify it, SHALL refuse to start if its own peer identity is not listed, and SHALL refuse to start as an orderer if its entry lacks the orderer role. A Station SHALL answer a roster request from any caller, without a handshake, with its current roster, or with an answer meaning it holds none. A Station without a roster SHALL behave exactly as it does today.

#### Scenario: The roster is served to a caller with only an address

- **WHEN** a Vessel that holds no roster sends a roster request to a Station started with one
- **THEN** it receives the roster, and the roster verifies against the Fleet identity the Vessel was given out of band

#### Scenario: A Station absent from its roster does not start

- **WHEN** a Station is started with a roster that does not list its peer identity
- **THEN** it exits with a non-zero status and an error naming the roster file

#### Scenario: Roles are enforced at start-up

- **WHEN** a Station is started with a fleet key file and a roster whose entry for it lacks the orderer role
- **THEN** it exits with a non-zero status naming the role mismatch

#### Scenario: No roster, no change

- **WHEN** a Station is started without a roster and receives a roster request
- **THEN** it answers that it holds none, and every other request behaves as before

### Requirement: A Station with a roster homes Territories fleeted

A Station started with a roster SHALL verify order proofs on ACL records it receives over sync against the roster's Fleet identity, refusing records whose proof does not verify, exactly as a fleeted Vessel does. Holds created before the roster was supplied SHALL be treated the same on the next start.

#### Scenario: A forged proof is refused by a Station

- **WHEN** a peer pushes an ACL record with a proof signed by a key other than the Fleet's to a Station started with that Fleet's roster
- **THEN** the Station's ACL log for that Territory does not grow

#### Scenario: A genuine proof is accepted

- **WHEN** the Fleet's orderer countersigns a record and a Vessel syncs it to a rostered storage Station
- **THEN** the Station accepts it and serves it to other members
