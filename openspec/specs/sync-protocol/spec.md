# sync-protocol Specification

## Purpose

Defines the contract between Vessels and Stations: the wire-level invariants of the sync protocol, and the Station's operational contract — how it is reached, how it is identified, how it admits callers, how it starts and stops, and what it records and reports about itself.

Throughout the specs, the **commodore** is the person who deploys and runs a Fleet, of any size: a self-hoster at a terminal, or whoever is on call for the hosted tier. The word is chosen so that _operator_ keeps its Kubernetes meaning and _Admin_ stays the Territory ACL's permission; neither is used for a person.

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

A Station SHALL bind its UDP port on every address family it serves and SHALL refuse to start if the port cannot be bound; it SHALL NOT fall back to a different port. The port is 6782 unless the commodore configures another. A configured port of 0 requests an ephemeral port, the posture for same-host development and tests.

At startup the Station SHALL publish the address it advertises where a commodore can read it without shell access to the data directory: in its log output as well as in the address file.

#### Scenario: A Vessel connects to a self-hosted Station

- **WHEN** a Vessel on a different host is configured with a Station's advertised address
- **THEN** it establishes a session with that Station

#### Scenario: Live delivery uses the Vessel's session

- **WHEN** a Station has a Notice to deliver to a subscribed Vessel
- **THEN** it is delivered over the session the Vessel opened
- **AND** the Station does not attempt to connect to the Vessel

#### Scenario: A configured port is the port that serves

- **WHEN** a Station is started with a port configured and a Vessel dials the Station's advertised address at that port
- **THEN** the session is established on that port

#### Scenario: The default port serves

- **WHEN** a Station is started with no port configured
- **THEN** it serves on UDP port 6782

#### Scenario: Port zero is ephemeral

- **WHEN** a Station is started with port 0 configured
- **THEN** it serves QUIC on a port the system assigned, and its published address names that port
- **AND** its probe routes answer over TCP at that same port

#### Scenario: A configured port is unavailable

- **WHEN** a Station is started with a port configured and another process already holds it
- **THEN** the Station exits with a non-zero status and an error naming the port
- **AND** it does not serve on any other port

#### Scenario: The advertised address is visible to the commodore

- **WHEN** a Station starts
- **THEN** its log output contains the complete advertised address a Vessel can be given, identical to the address file's content

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

### Requirement: A Station's identity key is provisioned from a file

A Station's identity is an Ed25519 key loaded from a key file. When the commodore names the key file explicitly, the file MUST exist: the Station SHALL load it and SHALL NOT create a key there, refusing to start with an error naming the path if it is absent or malformed. When no key file is named, the Station SHALL load the key at the default location inside its data directory, generating and persisting one there if none exists. A Station's authenticated endpoint identity SHALL be the same across restarts on the same key file.

#### Scenario: An explicitly named key file is missing

- **WHEN** a Station is started with a key file path that does not exist
- **THEN** the Station exits with a non-zero status and an error naming the path
- **AND** no file is created at that path

#### Scenario: A read-only key file is sufficient

- **WHEN** a Station is started with a key file that exists but is not writable
- **THEN** the Station starts and authenticates with the identity in that file

#### Scenario: The default location keeps development convenient

- **WHEN** a Station is started without naming a key file and its data directory holds no key
- **THEN** a key is generated and persisted in the data directory
- **AND** a second start without naming a key file presents the same identity

#### Scenario: Identity survives replacement of the process

- **WHEN** a Station is stopped and a new process is started on the same key file
- **THEN** a Vessel that trusted the previous process's identity establishes a session with the new one without any identity mismatch

### Requirement: The orderer role runs on a provisioned fleet key

A Station SHALL act as a Fleet's orderer exactly when it is supplied a fleet key file. The fleet key SHALL be loaded from that file and SHALL NOT be generated by the Station; a named fleet key file that is absent or malformed SHALL prevent the Station from starting. Order proofs issued by the orderer SHALL verify under the same fleet identity across restarts.

#### Scenario: Ordering authority persists across restarts

- **WHEN** an orderer countersigns an ACL record, is restarted on the same fleet key file, and a Vessel homed to that Fleet receives the record over sync
- **THEN** the record's order proof verifies

#### Scenario: A Station without a fleet key does not order

- **WHEN** a Station started without a fleet key file receives an order request
- **THEN** it answers with no proof

#### Scenario: A named fleet key file is missing

- **WHEN** a Station is started with a fleet key file path that does not exist
- **THEN** the Station exits with a non-zero status and an error naming the path

### Requirement: A Station stops cleanly on a termination signal

On receiving a termination signal (SIGTERM) or an interrupt (SIGINT), a Station SHALL stop accepting sessions, close its transport endpoint so that connected peers observe an orderly close rather than a timeout, and exit with status zero within a bounded time. The bound SHALL be short enough to fit inside a supervisor's ordinary grace period.

#### Scenario: A parked subscription observes the close

- **WHEN** a Vessel holds a live subscription to a Station and the Station receives SIGTERM
- **THEN** the Vessel's session ends with a transport close before its idle timeout would have expired
- **AND** the Station process exits with status zero

#### Scenario: An idle Station exits promptly

- **WHEN** a Station with no open sessions receives SIGTERM
- **THEN** it exits with status zero without waiting out any timeout

#### Scenario: Stored data is intact after a signalled stop

- **WHEN** a Station is stopped by SIGTERM and restarted on the same data directory
- **THEN** every Territory it homed is reloaded and served, with the Notices it had accepted present

### Requirement: A Station reports liveness and readiness over a probe listener

A Station SHALL serve plain HTTP over TCP at the same port number as its QUIC socket, on the wildcard address, with two paths: `/healthz`, which answers success whenever the process is running, and `/readyz`, which answers success only once the Station has finished loading its stored Territories and its transport endpoint is accepting sessions, and answers a non-success status before that. The listener is always present; a Station cannot be started without it. The two sockets SHALL share one port number in every mode: when the port is ephemeral, the Station SHALL choose a number that is free for both protocols, and its published address names that number. If a configured port cannot be bound for either protocol, the Station SHALL refuse to start. The listener SHALL expose nothing beyond these status answers: no Territory data, no key material, no addresses.

#### Scenario: Not ready while loading

- **WHEN** a Station is still loading its stored Territories
- **THEN** `/healthz` answers success and `/readyz` answers a non-success status

#### Scenario: Ready once serving

- **WHEN** the Station has loaded its stored Territories and is accepting sessions
- **THEN** `/readyz` answers success

#### Scenario: The probes share the Station's port number

- **WHEN** a Station is started on port 6782 and a prober connects to TCP port 6782
- **THEN** `/healthz` answers success over plain HTTP, while QUIC continues to be served on UDP port 6782

#### Scenario: The TCP side of the port is unavailable

- **WHEN** a Station is started with a port configured and another process already holds that port over TCP
- **THEN** the Station exits with a non-zero status and an error naming the port
- **AND** it does not serve QUIC without its probe routes

#### Scenario: The listener reveals nothing

- **WHEN** any path other than the two status paths is requested on the probe listener
- **THEN** the response is a not-found status with no body describing the Station

### Requirement: Station configuration is readable from the environment

Every Station start-up option SHALL be settable through an environment variable as well as a command-line flag, so a supervisor can configure a Station without composing a command line. When both are present, the command-line flag SHALL take precedence.

#### Scenario: Configuration through the environment alone

- **WHEN** a Station is started with no flags and its options set through environment variables
- **THEN** it starts with exactly the configuration those variables describe

#### Scenario: A flag overrides the environment

- **WHEN** an option is set both in the environment and as a flag
- **THEN** the flag's value is used

### Requirement: Station logs can be emitted as structured records

A Station SHALL offer a structured log format in which every log line is a single JSON object carrying at least a timestamp, a level, and a message, selectable at start-up. The default remains the human-readable format.

#### Scenario: Structured output

- **WHEN** a Station is started with the structured log format selected
- **THEN** every line it writes to its log output parses as a JSON object with timestamp, level, and message fields

#### Scenario: Default is unchanged

- **WHEN** a Station is started without selecting a log format
- **THEN** its log output is the human-readable format
