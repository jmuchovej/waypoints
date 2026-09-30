# sync-protocol Specification

## Purpose

TBD - created by archiving change sync-mark-graph. Update Purpose after archive.

## Requirements

### Requirement: The Notice header clock carries a hybrid logical clock value

A Notice's header clock SHALL be populated from the authoring Vessel's hybrid logical clock. The clock value SHALL be covered by the Notice signature and bound as associated data to the encrypted payload, so it cannot be altered in transit.

The clock SHALL remain advisory at the protocol layer: Notice validation SHALL NOT inspect it, admission SHALL NOT depend on it, and a Station SHALL NOT interpret it. Interpretation is reserved to Vessel-side consumers.

#### Scenario: An authored Notice carries a real clock value

- **WHEN** a Vessel authors a Notice
- **THEN** the header clock reflects that Vessel's hybrid logical clock rather than a fixed placeholder

#### Scenario: The clock cannot be altered in transit

- **WHEN** a relayed Notice's header clock is modified
- **THEN** signature verification fails and payload decryption fails

#### Scenario: Validation ignores the clock

- **WHEN** a Notice is validated
- **THEN** the outcome is unchanged for any header clock value, including values in the past or the future

#### Scenario: Observing a remote clock advances the local one

- **WHEN** a Vessel observes a Notice whose clock is ahead of its own
- **THEN** its subsequent authored clock values exceed the observed value

### Requirement: A Station is reachable at an advertised address

A Station SHALL bind a routable socket and SHALL advertise an address at which Vessels can reach it. A Station SHALL accept incoming sessions only; it SHALL NOT initiate outbound connections to Vessels.

#### Scenario: A Vessel connects to a self-hosted Station

- **WHEN** a Vessel on a different host is configured with a Station's advertised address
- **THEN** it establishes a session with that Station

#### Scenario: Live delivery uses the Vessel's session

- **WHEN** a Station has a Notice to deliver to a subscribed Vessel
- **THEN** it is delivered over the session the Vessel opened
- **AND** the Station does not attempt to connect to the Vessel

### Requirement: A Vessel binds a routable socket

A Vessel SHALL bind a socket from which outbound traffic can leave the host. A Vessel SHALL NOT be required to be reachable from outside its host in order to sync.

#### Scenario: A Vessel behind a NAT syncs

- **WHEN** a Vessel with no inbound reachability syncs with a Station at an advertised address
- **THEN** the sync succeeds, including live delivery of Notices pushed by the Station

### Requirement: Development builds do not repeatedly trigger host firewall authorization

A development build SHALL present a stable identity to the host operating system across rebuilds, so that a firewall authorization granted once continues to apply.

#### Scenario: Rebuilding does not re-prompt

- **WHEN** a developer rebuilds and reruns the application after authorizing it once
- **THEN** the host firewall does not prompt again

### Requirement: Sessions are kept alive and stale sessions are reaped

A session SHALL carry periodic liveness signalling. A Station SHALL treat a session that produces no liveness signal within an idle threshold as dead and SHALL release the resources and subscription state associated with it.

#### Scenario: A subscription outlives its Vessel

- **WHEN** a subscribed Vessel disappears without closing its session
- **THEN** the Station reaps the session once the idle threshold elapses
- **AND** no delivery state is retained for it

#### Scenario: An idle but live session survives

- **WHEN** a subscribed Vessel exchanges no Notices for longer than the idle threshold but continues liveness signalling
- **THEN** its subscription is retained

### Requirement: A Vessel detects a dead session and resumes

A Vessel SHALL detect that a session has stopped delivering and SHALL re-establish it. On reconnect it SHALL resume from its local cursor, receiving exactly the Notices it has not yet seen.

#### Scenario: A device sleeps and wakes on a different network

- **WHEN** a Vessel's session is severed by sleep or a network change
- **THEN** the Vessel re-establishes a session
- **AND** receives the Notices authored while it was disconnected, without redundant resupply of Notices it already holds

#### Scenario: Reconnect is not a special case

- **WHEN** a Vessel resumes after disconnection
- **THEN** the state it reaches is identical to the state reached by a Vessel starting cold and syncing once
