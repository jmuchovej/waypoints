# Spec Delta

## Purpose

States what license Waypoints is under and how that license is declared in the tree: the uniform policy, the license files and where each lives, where the "or later" election is made, which code is never a permissive candidate, and the rules a future permissive crate must satisfy at birth.

## ADDED Requirements

### Requirement: The project license is AGPL-3.0-or-later plus the App Store additional permission

The license of the Program SHALL be the GNU Affero General Public License, version 3 or (at the recipient's option) any later version, together with the additional permission under its section 7 that lets the Program be conveyed through application stores. The additional permission is a term of the license, not a side letter, and SHALL be granted to every recipient.

#### Scenario: A recipient reads the terms

- **WHEN** a recipient obtains the source tree or a binary distribution
- **THEN** the terms they find are AGPLv3 text, a notice electing "version 3 or any later version", and the application-store permission, and nothing else

#### Scenario: A hosted, modified Station

- **WHEN** a party modifies the Station and lets users interact with it over a network
- **THEN** section 13 obliges them to offer those users the modified Corresponding Source; nothing in the additional permission relaxes this

#### Scenario: Self-hosting an unmodified build

- **WHEN** a party hosts an unmodified Station for themselves or others
- **THEN** no obligation beyond the license's ordinary conveyance terms arises

### Requirement: The root LICENSE is the verbatim AGPLv3 text

The repository root SHALL hold one license file, `LICENSE`, containing the text of the GNU Affero General Public License version 3 exactly as published by the Free Software Foundation, with no edits, additions or removals. It SHALL be a regular file, not a symlink. The "or later" election SHALL NOT be written into it; it is made by the notices. Byte-level comparison against the published text is the test.

#### Scenario: Comparing against the published text

- **WHEN** the root `LICENSE` is compared byte for byte with the FSF's published plain-text AGPLv3
- **THEN** they are identical

#### Scenario: Someone proposes an edit to LICENSE

- **WHEN** a change to the repository alters the root `LICENSE` in any way other than replacing it with the FSF's published text
- **THEN** the change is refused

#### Scenario: The root is scanned for a license

- **WHEN** a tool that recognises licenses only from regular files in the repository root scans the repository
- **THEN** it finds exactly one license file and identifies AGPL-3.0

### Requirement: docs/licensing is the home of the licensing material

`docs/licensing/` SHALL hold every licensing document other than the root license text: `LICENSE.md`, the Free Software Foundation's own Markdown edition of AGPLv3 exactly as published, with no edits; `COPYING.exceptions`; and the contributor agreement `CLA.md`. No license text or agreement SHALL live anywhere else in the tree except third-party license files beside the third-party code they cover.

#### Scenario: A reader browses the directory

- **WHEN** a reader opens `docs/licensing/`
- **THEN** they find the AGPL text as a rendered Markdown page, the additional permission, and the contributor agreement, and nothing about licensing is elsewhere

#### Scenario: The Markdown edition is compared against the FSF's

- **WHEN** `docs/licensing/LICENSE.md` is compared byte for byte with the FSF's published Markdown edition of AGPLv3
- **THEN** they are identical

#### Scenario: The two editions say the same thing

- **WHEN** the root `LICENSE` and `docs/licensing/LICENSE.md` are compared word for word, ignoring Markdown heading and list markers
- **THEN** they are identical; they are two editions of one document, both the FSF's

### Requirement: COPYING.exceptions holds the application-store permission

`docs/licensing/COPYING.exceptions` SHALL hold the additional permission under AGPLv3 section 7 for conveyance through application stores. The permission SHALL excuse only the conflicts a store's non-negotiable terms create, conditioned on compliance with every license condition the store does not prevent and on the same version's Corresponding Source remaining available under the plain license through a channel the store does not control.

#### Scenario: Conveying through a store

- **WHEN** a party conveys the Program through a third-party store whose standard terms conflict with sections 6, 10 or 12 of the license
- **THEN** the conveyance is permitted to the minimum extent the store's terms require, provided Corresponding Source for that version is available under the plain license outside the store

#### Scenario: A self-operated channel

- **WHEN** a party operates their own distribution channel with restrictive terms they could negotiate
- **THEN** it is not an application store under the permission, and no condition of the license is excused

#### Scenario: Removing the permission downstream

- **WHEN** a downstream recipient conveys copies without the additional permission
- **THEN** that is permitted by section 7; it binds only their own copies and re-creates the store conflict only for them

#### Scenario: Notices and source are never excused

- **WHEN** a party relies on the permission
- **THEN** the obligations to make Corresponding Source available, to preserve copyright and license notices, and to impose no restriction of their own choosing remain in force

### Requirement: Every first-party workspace crate inherits the policy

The workspace manifest SHALL set `license = "AGPL-3.0-or-later"` once under `[workspace.package]`, and every first-party workspace crate SHALL inherit it with `license.workspace = true`. No first-party crate SHALL set its own `license` value. The expression SHALL be a valid, non-deprecated SPDX expression equal to the policy.

#### Scenario: A new first-party crate is added

- **WHEN** a crate is added to `[workspace] members` without a `license` value of its own
- **THEN** it inherits the policy and passes the manifest check

#### Scenario: A first-party crate overrides the license

- **WHEN** a first-party crate sets a `license` value different from the policy, or a deprecated identifier such as `AGPL-3.0`
- **THEN** the manifest check fails and names the crate

#### Scenario: Workspace crates are private

- **WHEN** the dependency license check runs
- **THEN** workspace crates are excluded from it (they are not published), while their license declarations still govern the distributed binaries and the source tree

### Requirement: The frontend is one work with the binary

Code under `src/` SHALL carry the project license. It is conveyed only inside the application, communicates with the Rust backend over a typed in-process IPC, and is built into the same bundle, so it is one work with the binary.

#### Scenario: A frontend file is examined

- **WHEN** a source file under `src/` is examined
- **THEN** it carries the project license notice, the same as a Rust source file

### Requirement: The sync crates are never a permissive candidate

`outpost-protocol`, `outpost-fabric`, `outpost-hold` and `outpost`, and any crate that carries Station or Vessel protocol, access-control, key-schedule, reconciliation or sync-storage logic, SHALL be under the project license regardless of any birth-policy argument.

#### Scenario: A proposal to make a sync crate permissive

- **WHEN** a change proposes a permissive license for any crate carrying sync logic
- **THEN** it is refused on this requirement, whatever interoperability argument accompanies it

### Requirement: Permissive is a birth policy, bounded by the layering rule

A workspace crate MAY be born under `MIT OR Apache-2.0` only if third parties must link it to interoperate with Waypoints. "Would be useful as a library" does not qualify. A crate SHALL NOT migrate from the project license to a permissive one. A permissive crate's normal dependency graph SHALL contain no copyleft workspace crate; dev- and build-dependencies are exempt.

#### Scenario: No permissive crate exists

- **WHEN** the layering check runs against a tree with no permissive workspace crate
- **THEN** it passes vacuously and is still run on every pull request

#### Scenario: A permissive crate depends on a copyleft crate

- **WHEN** a workspace crate declaring a permissive license has a workspace crate declaring the project license in its normal dependency graph
- **THEN** the layering check fails and names both crates

#### Scenario: A permissive crate integration-tests against the engine

- **WHEN** a permissive crate lists a copyleft workspace crate only under dev-dependencies
- **THEN** the layering check passes

#### Scenario: A permissive crate is born

- **WHEN** a crate is added with `license = "MIT OR Apache-2.0"` and a reason that third parties must link it
- **THEN** it carries `LICENSE-MIT` and `LICENSE-APACHE` in its directory, its files' headers match its manifest, and the manifest check accepts it as a declared tier rather than failing it against the policy

### Requirement: Generated and in-repo code follows the work it belongs to

Code generated from first-party sources SHALL carry the license of the work it is generated into. In-repo code distributed with the application SHALL carry the project license. In-repo code distributed separately SHALL be evaluated on its own and, until evaluated, SHALL sit under the project license by default. Where no rule clearly applies, the project license applies and the item is listed for the Steward. Ambiguity SHALL NOT resolve toward permissive.

#### Scenario: Generated TypeScript bindings

- **WHEN** bindings are generated from the Rust data model
- **THEN** they carry the project license, emitted by the generator

#### Scenario: A separately deployed surface

- **WHEN** a directory in the repository is deployed independently of the application
- **THEN** its code is first-party and carries the project license like any other; only its non-code content and brand material await the Steward's decision, sitting under the root `LICENSE` meanwhile

#### Scenario: First-party developer tooling

- **WHEN** a first-party crate or script under `tooling/` is examined
- **THEN** it carries the project license like any other first-party workspace member, even though it is never distributed with the application; third-party code under `tooling/` keeps its own license

#### Scenario: Media bundled into the application

- **WHEN** a first-party non-code asset — an icon, an image, a stylesheet's artwork — is bundled into the application binary
- **THEN** it carries the project license like the code around it; a Creative Commons or font license applies only where a file or directory says so, which is the case for documentation and website copy, not for bundled assets

### Requirement: Package metadata agrees with the license files

Every place the tree states a license outside the manifests — the Nix package metadata, the application bundle's copyright field, the README — SHALL agree with the root `LICENSE` and the notices. No surface SHALL state a license other than the project license for first-party code.

#### Scenario: Nix package metadata

- **WHEN** a first-party package is evaluated
- **THEN** its `meta.license` is the AGPLv3-or-later license object

#### Scenario: The bundle's copyright field

- **WHEN** the application bundle is inspected
- **THEN** its copyright field reads `Copyright © 2026 John Muchovej`, naming the holder the file headers name
