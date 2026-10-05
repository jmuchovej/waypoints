## Purpose

Defines how the repository's own CI and shells obtain credentials: where secret values live, how one declaration names them in more than one store, how a job reaches the credential that unlocks the store CI reads, how narrowly provider selection travels through a workflow, and how a contributor satisfies the same declarations without access to the maintainer's store.

## ADDED Requirements

### Requirement: Secrets are declared once, with coordinates per store

Every secret the repository consumes SHALL be declared exactly once, in one committed manifest, carrying its description and any encoding or path semantics. A declaration that is resolvable from more than one store SHALL carry each store's coordinates on that one declaration, keyed by the store's alias — the entry's title and, where the entry groups several values or carries attached files, the name of the component — never by a backend-specific path or URI. Adding or replacing a store SHALL add or replace coordinates; it SHALL NOT add a second declaration. A declaration SHALL NOT be retained for a value no store holds.

#### Scenario: Same declaration, two stores

- **WHEN** a secret carrying coordinates for two stores is resolved first with one store selected and then with the other
- **THEN** both resolutions succeed, each returning that store's value, with no change to the manifest between them

#### Scenario: Grouped entry resolves by component

- **WHEN** a profile references a value by an entry's title and a component name, and the store holds that entry with that component
- **THEN** the secret resolves to that component's value

#### Scenario: Attached text file resolves to its exact bytes

- **WHEN** a path-delivered secret references a text file attached to a store entry
- **THEN** the file the consumer receives is byte-identical to the attachment

#### Scenario: Binary material is carried as text

- **WHEN** a secret's value is not valid UTF-8 at rest
- **THEN** the store holds it encoded as text and the declaration names the encoding, so that the delivered file is byte-identical to the original

### Requirement: CI reads a vault; nothing committed holds secret material

No committed file SHALL contain a secret value in any form, encrypted or plain, and no committed file SHALL be required to decrypt one. Profiles that CI resolves SHALL read from an external vault authenticated by a credential held only in the CI platform's secret store. Rotating a secret SHALL be an edit to the vault entry and nothing else.

A local development store MAY exist on a contributor's or the maintainer's machine, encrypted to that person's own key. It SHALL be ignored by version control, SHALL NOT be read by any generated workflow, and SHALL be addressed from the same declarations CI uses.

#### Scenario: Rotation needs no commit

- **WHEN** a secret's value is replaced in the vault
- **THEN** the next workflow run resolves the new value, with no change to the repository

#### Scenario: Clone holds no secret material

- **WHEN** the repository is cloned at any commit from the cutover onward
- **THEN** the tree contains no encrypted secret store and no key material

#### Scenario: Local store stays local

- **WHEN** a development store file exists at its conventional path in a working copy
- **THEN** version control reports it as ignored, and no generated workflow references the key that would decrypt it

### Requirement: A job reaches a credential only through the environment it declares

Every generated job that resolves secrets SHALL declare the CI environment whose work it performs, and SHALL read the vault credential from that environment. No vault credential SHALL exist at repository scope. A job that resolves no secrets SHALL declare no environment and SHALL reference no credential. The environment a job declares SHALL follow from the work the job does, so that tightening an environment's protection or splitting one environment's credential from another's is a CI-platform change with no edit to the repository.

#### Scenario: Release job declares the release environment

- **WHEN** a generated job builds, signs, or publishes an application artifact
- **THEN** it declares the release environment, and its credential reference resolves against that environment's secrets

#### Scenario: Automation job declares the automation environment

- **WHEN** a generated job authenticates as the automation identity or plans a release
- **THEN** it declares the automation environment, and its credential reference resolves against that environment's secrets

#### Scenario: No repository-scoped credential

- **WHEN** the repository's secret store is inspected after cutover
- **THEN** no vault credential is defined at repository scope; every copy is scoped to an environment

#### Scenario: Secret-free job declares nothing

- **WHEN** a generated job contains no step that resolves secrets
- **THEN** it declares no environment and references no credential

### Requirement: Provider selection travels with the resolving step and no wider

A generated workflow SHALL NOT set a secret provider at workflow or job scope. Each step that resolves secrets SHALL carry its own provider selection and its own credential; every other step SHALL see neither, so that the shell's default provider holds wherever a secret is not being resolved.

#### Scenario: Non-resolving step inherits the shell default

- **WHEN** a step in a secret-resolving workflow runs a command that reads the manifest but resolves no vault-backed secret
- **THEN** that command uses the shell's default provider and succeeds without a vault client present

#### Scenario: Resolving step is self-describing

- **WHEN** a step that resolves a profile is inspected in the generated workflow
- **THEN** the step itself names the provider and supplies the credential, and the enclosing job and workflow name neither

### Requirement: A profile resolves exactly what its consumer needs

Resolving a CI-facing profile SHALL require only the secrets that profile exists to supply. The automation profile SHALL resolve only the automation identity's credentials and SHALL NOT require any secret declared for another purpose. The release profile, resolved under a platform scope, SHALL touch only that scope's secrets, and every secret the release profile declares SHALL belong to at least one scope.

#### Scenario: Automation resolves only its own secrets

- **WHEN** the automation profile is checked or exported
- **THEN** only the automation identity's secrets are required and read; no signing material and no developer token is consulted

#### Scenario: Scoped release resolution stays inside the scope

- **WHEN** the release profile is resolved under one platform's scope
- **THEN** only that platform's secrets are required and reach the build, and secrets outside the scope are absent from the build's environment

#### Scenario: No unscoped release secret

- **WHEN** the release profile's declarations are compared with the union of all platform scopes
- **THEN** every declared release secret appears in at least one scope

### Requirement: A contributor satisfies the same declarations from their own provider

A contributor SHALL be able to resolve any profile from a provider of their choosing — the local development store, a keyring, or a vault of their own — selected by environment or flag, without editing the committed manifest and without any access to the maintainer's vaults. Every build recipe SHALL honor that selection, including recipes that scrub the environment before building: those SHALL forward the provider selection and the credentials of the supported providers into the clean environment.

#### Scenario: Contributor builds a release from the local store

- **WHEN** a contributor encrypts values for the release profile's local-store coordinates to their own key, selects the local store, and runs a platform build recipe
- **THEN** the build resolves the contributor's values and never contacts the maintainer's vault

#### Scenario: Contributor builds a release from a keyring

- **WHEN** a contributor stores values for the release profile under a keyring provider and selects it
- **THEN** the build resolves those values from the keyring with no change to the manifest

#### Scenario: Clean-room recipe forwards the selection

- **WHEN** an environment-scrubbing build recipe runs with a provider selected in the caller's environment
- **THEN** the selection is in force inside the scrubbed environment, and the build does not fall back to the profile's default provider

### Requirement: Website at-rest secrets use the same resolver

The website's administrative secrets — the values rendered into its local dev-vars file and pushed to its runtime store — SHALL be declared as a profile in the same manifest, carrying coordinates for the vault CI will read and for the local development store, and resolved through the same resolver as every other profile. No recipe SHALL decrypt a store directly. The profile SHALL resolve only the website's own secrets, and SHALL declare exactly the values the website's runtime reads.

#### Scenario: Dev-vars render from the profile

- **WHEN** the website's dev-vars recipe runs
- **THEN** the dev-vars file is written from the website profile's resolved values, with the same keys the runtime store expects

#### Scenario: Runtime store is fed from the profile

- **WHEN** the website's secrets-push recipe runs
- **THEN** every secret in the website profile is uploaded to the runtime store under its declared name, and nothing is read from any other source

#### Scenario: Projection carries no decryption tool

- **WHEN** the toolchain the repository projects for CI is inspected
- **THEN** it provides the resolver and the vault client, and no standalone decryption tool

### Requirement: Every resolution states its reason

Every invocation that resolves secrets, in CI or in a recipe, SHALL state a reason for the access, and a resolution attempted without one SHALL be refused.

#### Scenario: Recipe states its reason

- **WHEN** any committed recipe or generated workflow step resolves secrets
- **THEN** the invocation carries an explicit reason describing why the values are needed

#### Scenario: Reasonless access is refused

- **WHEN** secrets are resolved with no reason supplied
- **THEN** the resolver refuses and names the missing reason
