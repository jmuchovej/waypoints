## MODIFIED Requirements

### Requirement: Reference manifests describe a one-Station Fleet that survives pod replacement

The repository SHALL carry reference Kubernetes manifests for a Fleet under `deploy/kubernetes/`: a StatefulSet of N storage Stations and a one-replica StatefulSet for the orderer, sharing one signed roster. Each Station SHALL have a single-writer block persistent volume at `/data`; each SHALL take its identity key from a Secret and never from the volume or the image; only the orderer SHALL mount the fleet key. The storage Stations' identities SHALL be an identity pool: a roster signed once for N Stations, with each pod selecting its key, its roster entry, and its advertised socket by its own ordinal, so that changing the replica count up to N re-signs nothing. Every pod SHALL declare the Station's port for both UDP and TCP, SHALL wire its startup, readiness, and liveness probes to the TCP side, SHALL have a termination grace period long enough for the Station's orderly close, and SHALL run as a non-root user with a read-only root filesystem. The manifests SHALL be usable at N = 1 with no orderer split for the one-Station Fleet they replace.

#### Scenario: Pod replacement preserves identity and data

- **WHEN** a Vessel has registered a Territory and synced Notices with a Station pod, and the pod is deleted
- **THEN** the replacement pod presents the same endpoint identity, serves the same Territory with the synced Notices present, and the Vessel syncs with it without reconfiguration

#### Scenario: Keys come from Secrets

- **WHEN** the manifests are inspected
- **THEN** each identity key and the fleet key are mounted from Secrets, no key is generated into a persistent volume, the fleet key is mounted only by the orderer, and no Secret contents are part of the repository

#### Scenario: Scaling within the pool re-signs nothing

- **WHEN** the storage StatefulSet is scaled from 1 to 3 within a roster signed for 3
- **THEN** the two new pods start on the second and third pooled identities, the roster is unchanged, and the Fleet converges on every Territory

#### Scenario: One port, two protocols

- **WHEN** the manifests are inspected
- **THEN** every container declares its port once for UDP and once for TCP, each Service exposes only the UDP side, and every probe targets the TCP side of the same number

#### Scenario: Readiness gates traffic during a slow start

- **WHEN** a Station pod starts with many stored Territories to load
- **THEN** the pod is not marked ready until the Station reports ready, and the startup probe allows the load to finish rather than restarting the pod

#### Scenario: Termination is orderly

- **WHEN** a pod is terminated by the orchestrator while a Vessel holds a live subscription
- **THEN** the Vessel observes an orderly session close, and the container exits within the grace period without being killed

#### Scenario: A killed Station loses nothing

- **WHEN** a Territory and its Notices are on a three-Station Fleet and one storage pod's volume is destroyed with the pod
- **THEN** the Territory is still served by the others, and the pod that replaces the destroyed one backfills it without any Vessel's involvement

## ADDED Requirements

### Requirement: Each Station has its own advertised socket

Every Station in the reference Fleet SHALL be reachable at an address of its own, routed to that pod alone, because a Vessel verifies the identity it dials against the identity it expected and a shared address would land it on the wrong Station. The manifests SHALL route each Station's UDP port through a per-Station Service, and each pod SHALL advertise the socket the commodore assigns to its ordinal. The roster's entry for each Station SHALL name that socket.

#### Scenario: Dialing an entry reaches that Station

- **WHEN** a Vessel dials the socket in a roster entry
- **THEN** the Station that answers presents that entry's identity, every time

#### Scenario: No shared address is offered

- **WHEN** the manifests are inspected
- **THEN** no Service selects more than one Station pod
