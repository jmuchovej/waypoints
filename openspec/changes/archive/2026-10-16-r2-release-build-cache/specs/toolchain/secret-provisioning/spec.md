## MODIFIED Requirements

### Requirement: A profile resolves exactly what its consumer needs

Resolving a CI-facing profile SHALL require only the secrets its consumer needs. Two profiles face CI. The release profile holds signing material and SHALL be resolved only under a platform scope, touching only that scope's secrets. The ci profile holds every non-signing credential CI uses — the automation identity's credentials and the build-cache store's credential among them — grouped into one scope per consumer, and SHALL be resolved only under one of those scopes; no generated workflow SHALL resolve it unscoped. Every secret either profile declares SHALL belong to at least one of that profile's scopes.

#### Scenario: Automation resolves only its own secrets

- **WHEN** a generated job authenticates as the automation identity
- **THEN** it resolves the ci profile under the automation identity's scope, and only that identity's secrets are required and read; no store credential, no signing material, and no developer token is consulted

#### Scenario: Build-cache store resolves under its own scope

- **WHEN** a generated job contacts the build-cache store
- **THEN** it resolves the ci profile under the store's scope, and only the store's secrets are required and read; no automation credential and no signing material is consulted

#### Scenario: Scoped release resolution stays inside the scope

- **WHEN** the release profile is resolved under one platform's scope
- **THEN** only that platform's secrets are required and reach the build, and secrets outside the scope are absent from the build's environment

#### Scenario: No unscoped resolution of a CI-facing profile

- **WHEN** the generated workflows are inspected
- **THEN** every resolution of the release profile and every resolution of the ci profile names a scope

#### Scenario: No unscoped release secret

- **WHEN** the release profile's declarations are compared with the union of all platform scopes
- **THEN** every declared release secret appears in at least one scope

#### Scenario: No unscoped ci secret

- **WHEN** the ci profile's declarations are compared with the union of its consumer scopes
- **THEN** every declared ci secret appears in at least one scope
