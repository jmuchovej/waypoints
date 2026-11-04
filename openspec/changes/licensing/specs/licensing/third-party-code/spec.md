# Spec Delta

## Purpose

Governs code the project did not write: which dependency licenses are acceptable in the Rust and frontend builds, how third-party code copied into the tree is labelled and attributed, and the third-party notices artifact — what it covers, how it is kept honest, and where it ships.

## ADDED Requirements

### Requirement: Dependency licenses come from the compatible allowlist

Every dependency reaching a distributed build SHALL satisfy an allowlist of licenses compatible with AGPLv3: `MIT`, `MIT-0`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`, `CC0-1.0`, `MPL-2.0`, `Unlicense`, `0BSD`, `BSL-1.0`, `NCSA`, `CDLA-Permissive-2.0`. A dependency whose expression no allowlisted identifier satisfies is a blocking finding. Workspace crates are excluded, being unpublished.

#### Scenario: A new dependency under an unlisted license

- **WHEN** a dependency enters the lockfile whose license expression no allowlisted identifier satisfies
- **THEN** the pull-request check fails naming the crate and its expression

#### Scenario: A choice-of-license dependency

- **WHEN** a dependency declares `MIT OR EUPL-1.2`
- **THEN** it passes, because `MIT` satisfies the expression; the unlisted alternative is never relied on

#### Scenario: The frontend bundle

- **WHEN** the production frontend bundle is built
- **THEN** every package bundled into it satisfies the same allowlist, and the build fails on one that does not

### Requirement: The Rust and frontend allowlists are one list

The frontend build's allowlist SHALL equal the Rust dependency checker's allowlist. Divergence SHALL fail the licensing check on the pull request.

#### Scenario: One side is edited

- **WHEN** an identifier is added to one allowlist and not the other
- **THEN** the licensing check fails naming the identifier and which side lacks it

### Requirement: Bad metadata is clarified per package, never suppressed

Where a dependency's declared license is missing, malformed or contradicted by its license files, the correction SHALL be a per-package clarification naming the package, its version where the tooling supports it, and the expression established from its license files. The check SHALL NOT be weakened for the whole graph to accommodate one package.

#### Scenario: A crate with a missing license field

- **WHEN** a Rust dependency ships no `license` field but a recognisable license file
- **THEN** a clarification entry for that crate, pinned to its license file's content, makes it pass, and the entry is reviewed when the crate's version changes

#### Scenario: A frontend package with malformed metadata

- **WHEN** a bundled package's `license` field is not a valid expression
- **THEN** an override entry keyed by `name@version` supplies the expression, and the bundle gate passes on that entry alone

### Requirement: Third-party code in the tree declares and carries its own license

Code copied into the repository — a vendored crate, a `[patch.crates-io]` fork, a ported algorithm, a copied snippet — SHALL keep its upstream license and attribution. A vendored or patched crate SHALL set its own `license` value rather than inheriting the workspace's, and its directory SHALL hold the upstream license text and any upstream notice file. Its license SHALL be on the compatible allowlist. A modified MPL-2.0 file SHALL stay MPL-2.0 and SHALL be published.

#### Scenario: The vendored workflow generator

- **WHEN** the manifest of the vendored GitHub Actions workflow crate is read
- **THEN** it declares `license = "Apache-2.0"` directly, and its directory holds the upstream `LICENSE`; upstream ships no `NOTICE`, so there is none to carry

#### Scenario: A patched registry crate

- **WHEN** a crate under `[patch.crates-io]` is examined
- **THEN** its manifest declares the upstream license and its directory holds the upstream license text

#### Scenario: A vendored crate is re-synced from upstream

- **WHEN** the sync recipe overwrites a vendored crate from upstream
- **THEN** the license declaration and the license and notice files survive the sync

#### Scenario: Third-party code under an incompatible license

- **WHEN** code under a license not on the allowlist is proposed for the tree
- **THEN** the change is refused

#### Scenario: Attribution lives beside the code

- **WHEN** a third-party license text is added for code in the tree
- **THEN** it sits in that code's own directory, not under `docs/licensing/`, which holds only the project's own licensing material

### Requirement: One registry records every piece of third-party code

The tree SHALL hold one registry of third-party code — vendored crates, copied files, and ported portions — recording each item's path, origin, copyright holder, license and license-text files. The manifest check, the header check's exemptions and the notices SHALL all derive from it. Every registered license SHALL be on the compatible allowlist and every named text file SHALL exist.

#### Scenario: A registered third-party crate

- **WHEN** a workspace crate registered as third-party declares `Apache-2.0`
- **THEN** the manifest check passes it, and the header check skips its files

#### Scenario: An unregistered crate declaring another license

- **WHEN** a workspace crate that is neither registered as third-party nor born as a permissive tier declares `MIT`
- **THEN** the manifest check fails naming it and the two ways it could be made conforming: register it as third-party, or inherit the policy

#### Scenario: A ported portion

- **WHEN** a routine ported from another project is registered as a portion with its origin, holder and license
- **THEN** the notices file carries that attribution in a hand-authored section ahead of the dependency sections, and the containing file's own license is unchanged

#### Scenario: A registered license off the allowlist

- **WHEN** a registry entry names a license no allowlisted identifier satisfies
- **THEN** the licensing check fails naming the entry

#### Scenario: A missing license text

- **WHEN** a registry entry names a license-text file that does not exist at the entry's path
- **THEN** the licensing check fails naming the entry and the file

### Requirement: A third-party notices file is generated and committed

The repository SHALL commit a `THIRD-PARTY-NOTICES.md` listing every Rust dependency of the distributed build targets and every production dependency of the frontend, with each package's name, version, license expression and license text. The file SHALL be produced by one command from the two lockfiles, and the generated-file lockstep check SHALL fail when the committed file differs from regeneration.

#### Scenario: A dependency changes

- **WHEN** either lockfile changes and the notices file is not regenerated in the same change
- **THEN** the lockstep check fails naming the notices file

#### Scenario: A dependency without a license text

- **WHEN** a Rust dependency's license text cannot be located for the notices
- **THEN** generation fails naming the crate, until a clarification supplies the text

#### Scenario: Regeneration is idempotent

- **WHEN** the notices command runs twice against unchanged lockfiles
- **THEN** the second run produces no difference

### Requirement: The notices ship with the application

The notices file SHALL be bundled as an application resource, reachable from the application's settings, and SHALL be included in every release archive together with the root `LICENSE` and `COPYING.exceptions`.

#### Scenario: A user looks for the licenses of what they installed

- **WHEN** a user opens the licenses entry under the application's settings
- **THEN** they see the committed notices file for the version they are running

#### Scenario: A release archive

- **WHEN** a release archive for any platform is inspected
- **THEN** it contains `THIRD-PARTY-NOTICES.md` alongside `LICENSE` and `COPYING.exceptions`

### Requirement: The dependency license check runs on every pull request

The Rust dependency license check SHALL run on every pull request and before every push, with the same configuration in both places.

#### Scenario: A pull request adds a dependency

- **WHEN** a pull request changes `Cargo.lock`
- **THEN** the dependency license check runs in the pull request's checks and its result gates the merge
