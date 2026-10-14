# distribution/station-deployment Specification

## Purpose

Defines the deployable form of a Station: the container image Nix builds from the `outpost` package, the guarantees that image makes to whoever runs it, and the reference Kubernetes manifests for a one-Station Fleet that self-hosters and the hosted tier both start from.

## Requirements

### Requirement: The Station image is a Nix build

The repository SHALL provide a container image of the Station as a flake package for Linux systems, built by Nix from the `outpost` package without a Dockerfile. The image SHALL NOT be part of the overlay's package set. The image's entrypoint SHALL run the Station, with `/data` as its data directory and declared as a volume, so that running the image with a volume at `/data` and no further arguments starts a Station that persists there.

#### Scenario: Image builds from the flake

- **WHEN** a user runs `nix build <flake-ref>#outpost-image` on a Linux system
- **THEN** the output is a container image archive loadable by an OCI-compatible runtime

#### Scenario: Image runs a Station by default

- **WHEN** the image is run with a writable volume mounted at `/data` and no arguments
- **THEN** a Station starts, creates its data layout under `/data`, and publishes its address

#### Scenario: Overlay is unaffected

- **WHEN** a consumer applies the overlay
- **THEN** no image attribute is added to their package set, and the overlay's inputs are unchanged

### Requirement: The image runs unprivileged and needs no egress

The image SHALL run the Station as a non-root user and SHALL work with a read-only root filesystem, writing only under `/data`. The image SHALL NOT require a certificate bundle, DNS, or any outbound network access to serve Vessels: the Station's only network requirement is inbound traffic on its port, over UDP for QUIC and over TCP for the probe routes.

#### Scenario: Non-root with read-only root

- **WHEN** the image is run with a read-only root filesystem, as a non-root user, with a writable volume at `/data`
- **THEN** the Station starts and serves

#### Scenario: Serving with no egress

- **WHEN** the image runs in a network namespace that permits only inbound traffic to its port over UDP and TCP
- **THEN** a Vessel registers a Territory and syncs with it

### Requirement: Reference manifests describe a one-Station Fleet that survives pod replacement

The repository SHALL carry reference Kubernetes manifests for a one-Station Fleet under `deploy/kubernetes/`. The manifests SHALL give the Station a single-writer block persistent volume at `/data`, SHALL supply its identity key and fleet key from Secrets rather than from the volume or the image, SHALL declare the Station's port for both UDP and TCP, SHALL route the UDP side through a Service, SHALL wire the pod's startup, readiness, and liveness probes to the TCP side of that same port, and SHALL grant a termination grace period long enough for the Station's orderly close. The manifests SHALL run the container as a non-root user with a read-only root filesystem.

#### Scenario: Pod replacement preserves identity and data

- **WHEN** a Vessel has registered a Territory and synced Notices with the Station pod, and the pod is deleted
- **THEN** the replacement pod presents the same endpoint identity, serves the same Territory with the synced Notices present, and the Vessel syncs with it without reconfiguration

#### Scenario: Keys come from Secrets

- **WHEN** the manifests are inspected
- **THEN** the identity key and fleet key are mounted from Secrets, no key is generated into the persistent volume, and the Secret contents are not part of the repository

#### Scenario: One port, two protocols

- **WHEN** the manifests are inspected
- **THEN** the container declares its port once for UDP and once for TCP, the Service exposes only the UDP side, and every probe targets the TCP side of the same number

#### Scenario: Readiness gates traffic during a slow start

- **WHEN** a Station pod starts with many stored Territories to load
- **THEN** the pod is not marked ready until the Station reports ready, and the startup probe allows the load to finish rather than restarting the pod

#### Scenario: Termination is orderly

- **WHEN** the pod is terminated by the orchestrator while a Vessel holds a live subscription
- **THEN** the Vessel observes an orderly session close, and the container exits within the grace period without being killed

### Requirement: The advertised address is the operator's to supply

The manifests SHALL NOT invent the address Vessels dial. The Station is configured to advertise an operator-supplied address, set after the Service has been assigned one, and the manifests SHALL document that step at the point of configuration. This is the no-relay reachability posture: each Station has a stable public UDP endpoint of its own.

#### Scenario: Operator completes the address

- **WHEN** the Service has an external address and the operator sets it as the Station's advertised address
- **THEN** the Station's published address names that external socket, and a Vessel given the published address syncs with the pod

#### Scenario: Address absent

- **WHEN** the manifests are applied without an advertised address
- **THEN** the Station still starts and serves, publishing only addresses that are reachable from inside the cluster

### Requirement: Manifests validate against the Kubernetes API schema

The reference manifests SHALL render with kustomize and SHALL validate against the Kubernetes API schema with a schema validator in strict mode, so that a drift in field names or types fails before any cluster sees it. The validation SHALL be runnable from the repository's task runner.

#### Scenario: Validation passes

- **WHEN** the manifests are rendered and validated with the repository's task
- **THEN** validation reports no errors

#### Scenario: A malformed field fails

- **WHEN** a manifest field is misspelled or given a value of the wrong type
- **THEN** validation fails naming the resource and field
