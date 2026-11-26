# Spec Delta

## MODIFIED Requirements

### Requirement: Reference manifests describe a one-Station Fleet that survives pod replacement

The repository SHALL carry reference Kubernetes manifests for a one-Station Fleet under `distribution/kubernetes/`. The manifests SHALL give the Station a single-writer block persistent volume at `/data`, SHALL supply its identity key and fleet key from Secrets, not from the volume or the image, SHALL declare the Station's port for both UDP and TCP, SHALL route the UDP side through a Service, SHALL wire the pod's startup, readiness, and liveness probes to the TCP side of that same port, and SHALL grant a termination grace period long enough for the Station's orderly close. The manifests SHALL run the container as a non-root user with a read-only root filesystem.

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

#### Scenario: The overlay finds the base

- **WHEN** the kind overlay under `distribution/kubernetes/kind/` is rendered with kustomize
- **THEN** it resolves the base manifests from `distribution/kubernetes/base/` by relative path and renders without error
