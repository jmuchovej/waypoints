# licensing/project-license Specification

## Purpose

States what license Waypoints is under and how that license is declared in the tree: the uniform policy, the license files and where each lives, where the "or later" election is made, which code is never a permissive candidate, and the rules a future permissive crate must satisfy at birth.

## Requirements

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

`docs/licensing/` SHALL hold every licensing document other than the root license text: `LICENSE.md`, the Free Software Foundation's own Markdown edition of AGPLv3 exactly as published, with no edits; `COPYING.exceptions`; the contributor agreement `CLA.md`; and the generated `THIRD-PARTY-NOTICES.md`. No license text or agreement SHALL live anywhere else in the tree except third-party license files beside the third-party code they cover.

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

## Decisions

### 2026-08-28 — AGPL-3.0-or-later for the whole tree

**Decision.** One license, `AGPL-3.0-or-later`, set once in the workspace manifest and inherited by every first-party crate and by the in-repo frontend.

**Rationale.** The sync Station is an accept-only, content-blind network service. Anyone hosting a modified Station has users interacting with it over a network, which is the trigger of AGPLv3 section 13, so the copyleft lands on the one component whose hosted modification is the competitive risk. Static linking makes copyleft whole-binary anyway, so a per-crate split buys nothing. "Or later" gives downstream the same migration option the Steward keeps through the contributor agreement. The frontend is one work with the binary — intimate IPC, single distribution — and licensing it separately would invite ambiguity at the webview boundary.

**Rejected alternatives.**

- _SSPL, BUSL, FSL for the core._ Not open source; forecloses the fork guarantee the project has made in public.
- _LGPL, MPL-2.0 for the core, plain GPL-3.0._ No network clause; a hosted modified Station would owe nothing.
- _`AGPL-3.0-only`._ Denies downstream the later-version option while the Steward keeps a relicensing path.

### 2026-08-28 — The sync crates are never permissive

**Decision.** `outpost-protocol`, `outpost-fabric`, `outpost-hold` and `outpost` stay under the project license whatever interoperability argument is made.

**Rationale.** The protocol crate is not thin wire types: it is the access-control fold, the key schedule, validation and reconciliation and the full cryptographic suite. Because the Station is content-blind, that crate is the hard majority of a clean-room Station, and a permissive copy would arm a hosted competitor against section 13. Third-party clients remain possible as AGPL clients.

**Rejected alternatives.**

- _A permissive protocol crate for third-party clients._ If proprietary third-party clients ever matter, the surgical answer is a new thin crate holding framing and message types only, never loosening the existing stack.

### 2026-08-28 — Permissive is a birth policy, never a migration; the layering check lands now

**Decision.** A crate may be born `MIT OR Apache-2.0` only if third parties must link it to interoperate. No existing crate qualifies, so no crate is permissive. The layering check (no copyleft workspace crate in a permissive crate's normal dependency graph) runs from day one and passes vacuously.

**Rationale.** Reversibility. While copyright is consolidated, AGPL to permissive is a free later flip; permissive to AGPL cannot recall shipped versions. The layering check costs nothing while vacuous and is already live when the first permissive crate appears, so that crate cannot be born wrong. The `bedrock` family has no standalone-crate ambition and stays internal.

**Rejected alternatives.**

- _`MIT` alone for a permissive crate._ No patent grant.
- _`Apache-2.0` alone._ GPLv2-incompatible; the dual is the Rust convention.
- _Marking candidate crates permissive now "to be ready"._ Nothing qualifies, and the flip is free later.

### 2026-08-31 — A true section 7 additional permission for application stores

**Decision.** `COPYING.exceptions` is a genuine AGPLv3 section 7 additional permission, granted to every recipient, forks included, conditioned on a restriction-free source channel for the same version.

**Rationale.** Three genres exist. A non-objection waiver (the `mosh` `COPYING.iOS` lineage, used by Nextcloud iOS) is a personal covenant: it neither travels with the code nor binds a successor cleanly, and the contributor agreement's Project License mechanism needs the permission to be an actual license term so that contributed code can never lose store distributability. An alternative-license conveyance (WireGuard iOS, conveying under MPL-2.0 through the store) splits license identity and fits poorly beside the AGPL server story. The section 7 permission (the Feeel and wger genre) travels with the code, binds successors through the agreement, and costs section 13 nothing — it addresses conveyance-through-stores frictions only. The universal grant is legally cleanest and pledge-consistent; trademark handles store-listing confusion. "Operated by a third party not under your control" plus "terms you cannot individually negotiate" forecloses a construct-your-own-store reading. Crate metadata carries the plain SPDX identifier because no registered SPDX exception exists for store permissions; the permission is strictly broadening, so metadata understates nothing a recipient must know.

**Rejected alternatives.** The waiver and the alternative-license genres, above.

### 2026-11-03 — Two FSF editions: the plain text at the root, the Markdown under `docs/licensing/`

**Decision.** The root `LICENSE` is the FSF's plain-text edition of AGPLv3, verbatim, the only license file at the root. `docs/licensing/LICENSE.md` is the FSF's Markdown edition of the same text, verbatim. `docs/licensing/` is the browsable home of the licensing material — `LICENSE.md`, `COPYING.exceptions`, `CLA.md`, `STEWARD.md`, the `README.md` overview — and is authored to render as pages, because `docs/` is the source of what the website publishes. Both editions are hash-checked against the FSF's published files; neither is a symlink.

**Rationale.** GitHub detects a repository's license with `licensee`, which scores files in the repository root only; its git-backed project keeps symlink entries as blobs and reads the blob at the entry's object id — for a symlink, the target path — so a symlinked root license file is not detected (licensee issues #155 and #227, both closed without a fix; the documented workaround is a real copy), and a project with two root files matching known licenses gets no license at all. So the detected file has to be a regular file at the root, alone. Run locally on 2026-11-03 (`licensee` 9.18.0), the plain-text edition matches AGPL-3.0 at 100% through the exact matcher, while the Markdown edition matches at 99.17% through the similarity matcher against a 98% threshold — detected, but by a margin that depends on the detector's stripping of `#` headings, list markers and emphasis, which regressed once (its issue #435, 2020) before being fixed. The root therefore carries the plain text and keeps the exact match. The website wants the license as a rendered page, and the FSF publishes its own Markdown edition at `https://www.gnu.org/licenses/agpl-3.0.md` (alongside `txt`, `html`, `texi`, `tex`, `rtf` and `dbk`; each is the FSF's own verbatim document), so the rendered copy is also verbatim rather than a conversion; a `.md` file renders where a `.md` symlink to plain text would reflow it. Two copies of one document cost 34 KB and a hash check each, and cannot drift because neither may be edited. The GPL-family convention is `COPYING`; detection accepts either name, and `LICENSE` was preferred for readers who do not know the convention. The section 7 permission works wherever it lives as long as the notices name the path, so it sits with the rest under `docs/licensing/`.

**Rejected alternatives.**

- _One file: the FSF Markdown edition at the root, with `docs/licensing/LICENSE.md` a symlink to it._ Sound: one verbatim document, detected at 99.17%. Rejected only for the margin — the plain text's exact match does not depend on normalisation behaviour.
- _A root `COPYING` or `LICENSE` symlink into `docs/licensing/`._ Not detected; the repository would show no license.
- _Every license file at the root._ Three root files for a flat-root repository, and a second-match risk if `COPYING.exceptions` is ever scored as a license text.
- _`COPYING` as the root name._ Equivalent for detection; `LICENSE` reads better to non-GNU audiences, and `COPYING.exceptions` keeps its name because it names what it is.
- _A third-party Markdown conversion of the text._ Not needed: the FSF's own Markdown edition exists and is verbatim by definition.

### 2026-11-02 — First-party developer tooling stays under the policy

**Decision.** First-party code under `tooling/` — `xtask`, the `bin/` scripts, `infra/` — carries the project license like every other first-party workspace member. Decided by the Steward on being asked; it is not a default left unexamined.

**Rationale.** Tooling is never conveyed except as part of the source tree, which is under the project license as a whole, so the copyleft on it constrains nobody who builds or runs the application; its only effect is on someone lifting the tooling's code into another project. A permissive tooling tier would also need the layering rule narrowed: `xtask` links `cartograph` and `domain` for typegen, the dev server and the demo territory, so a permissive `xtask` has copyleft workspace crates in its normal dependency graph. Narrowing that rule to crates third parties link is coherent but adds a third classification to the validator for no present need, and the flip from the project license to permissive stays free while copyright is consolidated.

**Rejected alternatives.**

- _A permissive tooling tier (`MIT OR Apache-2.0` for first-party `tooling/`)._ Liftable tooling, at the cost of a third validator classification, per-crate license texts under `tooling/`, and a layering rule scoped to the SDK tier. Available later as a free flip if the tooling is ever worth publishing on its own.
- _Leaving `tooling/` without headers, as a pending surface._ It is not pending; nothing about it awaits a decision now that one has been made.

### 2026-11-04 — Bundled media defaults to the project license; a short menu for everything else

**Decision.** Any non-code asset bundled into the application binary carries the project license. The contributor agreement lets the Steward place contributed Media under the Project License, `CC-BY-4.0`, `CC-BY-SA-4.0`, `CC0-1.0`, or, for fonts only, the SIL Open Font License 1.1; the Creative Commons options are for documentation and the website.

**Rationale.** An asset compiled into the binary is part of the one work the license covers, and giving it a different license would reintroduce the per-component ambiguity the uniform policy exists to avoid. Documentation and website copy are read, not linked, and the Creative Commons family is what readers and republishers expect there; a font license exists because fonts are the one asset class whose own community licenses expect to be used as-is.

**Rejected alternatives.**

- _Creative Commons for bundled assets._ Splits the binary's license by file type for no one's benefit.
- _No Media list, Project License only._ Leaves documentation under a software license its readers do not recognise.
