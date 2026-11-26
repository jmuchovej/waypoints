# licensing/third-party-code Specification

## Purpose

Governs code the project did not write: which dependency licenses are acceptable in the Rust and frontend builds, how third-party code copied into the tree is labelled and attributed, and the third-party notices artifact — what it covers, how it is kept honest, and where it ships.

## Requirements

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
- **THEN** it sits in that code's own directory, not under `docs/licensing/`, which holds the project's own licensing material and the notices generated from these texts

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

The repository SHALL commit a `THIRD-PARTY-NOTICES.md` listing every Rust dependency of the distributed build targets and every dependency of the frontend package, dev included since the bundler reads them, with each package's name, version, license expression and license text. One command SHALL produce it, and the generated-file lockstep check SHALL fail when the committed file differs from regeneration.

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

The notices file SHALL be bundled as an application resource together with the root `LICENSE` and `COPYING.exceptions`, so every installer carries all three, and SHALL be reachable from the application's settings.

#### Scenario: A user looks for the licenses of what they installed

- **WHEN** a user opens the licenses entry under the application's settings
- **THEN** they see the committed notices file for the version they are running

#### Scenario: An installer

- **WHEN** an installer for any platform is inspected
- **THEN** its resources contain `THIRD-PARTY-NOTICES.md` alongside `LICENSE` and `COPYING.exceptions`

### Requirement: The dependency license check runs on every pull request

The Rust dependency license check SHALL run on every pull request and before every push, with the same configuration in both places.

#### Scenario: A pull request adds a dependency

- **WHEN** a pull request changes `Cargo.lock`
- **THEN** the dependency license check runs in the pull request's checks and its result gates the merge

## Decisions

### 2026-08-28 — Division of labour: cargo-deny, a bundle plugin, cargo-about, and one first-party validator

**Decision.** Dependency allowlisting for Rust is `cargo-deny`'s; for the frontend bundle it is a Vite-side license plugin with the same allowlist; notice generation is `cargo-about` plus a walk of the frontend's installed packages; the first-party validator checks only what none of those do — manifests against policy, headers against manifests, layering, the license texts against the FSF's, the registry's consistency, and that the two allowlists are one.

**Rationale.** Nothing should reimplement `cargo-deny`: it already has expression evaluation, text detection and the clarification machinery, and the tree already carries a tuned configuration. The frontend gate operates on the actual bundle, the legally relevant set, and is package-manager-agnostic. The validator fills the gaps and reports every failure in one run.

**Rejected alternatives.**

- _One tool for everything._ Would reimplement allowlisting badly.
- _Lockfile-only gating for the frontend._ The gate should see what ships; the bundle plugin does, at build time.

### 2026-11-02 — In-tree third-party code declares its own license

**Decision.** The vendored workflow generator declares `Apache-2.0` itself and carries upstream's `LICENSE`; the two patched registry crates carry `LICENSE-MIT`; the validator learns which workspace crates are third-party from the registry.

**Rationale.** The vendored crate inherits the workspace license field today. After the flip that would label another party's Apache-2.0 code as AGPL — false, and a gap against Apache-2.0's notice-preservation condition. Third-party code is not ours to relabel; the compatible-license condition is what lets it be combined with ours, and that is what the check verifies for those crates.

**Rejected alternatives.**

- _Re-vendor with our headers._ Upstream has none, and adding ours would be the same mislabel.
- _Treat the vendored crate as first-party because of the local delta._ The delta is a modification of their work under their license, not a new work.
- _A central `docs/licensing/LICENSE.APACHE` for the vendored crate's license._ Attribution belongs beside the code it covers, and a directory of every third-party license text is what REUSE's `LICENSES/` is for, which is deferred.

### 2026-11-02 — Notices are generated from the lockfiles and committed

**Decision.** `THIRD-PARTY-NOTICES.md` is generated by one command from the third-party registry, `Cargo.lock` (via `cargo-about`) and the frontend's installed packages (walked from `src/package.json`, pinned by `pnpm-lock.yaml`), committed, and covered by the generated-file lockstep check.

**Rationale.** Committing generated output and lockstep-checking it is how every other generated file in the repository is kept honest, and it makes a dependency change visible as a notice change in review. A list derived from what is installed may include a package the bundler tree-shook, which costs nothing; it never omits one, which is what the MIT, ISC and BSD notice-preservation conditions care about. The registry supplies what no lockfile can: attribution for code ported into first-party files.

**Rejected alternatives.**

- _Build-time generation from the bundle plugin's bill of materials._ Exact to the shipped byte, but it needs a full frontend build inside the lockstep check and a `cargo-about` run inside the Nix derivation, where registry sources are not laid out as the tool expects.
- _A separate notices file per ecosystem._ Two files to ship and surface for no gain.

### 2026-11-04 — One registry of in-tree third-party code

**Decision.** `.config/third-party.toml` records every vendored crate, copied file and ported portion with its path, origin, holder, license and license-text files. The header check's exemptions, the manifest check's third-party set and the notices all derive from it; every registered license must be on the allowlist and every named text must exist.

**Rationale.** Three consumers want the same facts, and a lockfile-driven notices generator can never see a routine ported into a first-party file. One data file the Steward edits replaces two constants in the validator and closes the notices gap, and "one list" — the allowlist cargo-deny and the Vite gate already share — extends to in-tree code by construction.

**Rejected alternatives.**

- _Constants in the validator for exempt paths and third-party crates._ Two lists kept in step by hand, and no path from a ported snippet to the notices.
- _REUSE `.license` sidecars as the record._ They mark files, not portions, and REUSE is deferred.
