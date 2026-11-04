# Spec Delta

## Purpose

States what a contributor grants and what they are owed in return: the contributor license agreement's load-bearing clauses as invariants, the public pledge that quotes them, the enforcement-intent statement, and the rule that every contribution lands with recorded assent to a specific version of the agreement.

## ADDED Requirements

### Requirement: Contributions are accepted under the contributor license agreement

Every contribution SHALL be accepted under the Waypoints Individual Contributor License Agreement at `docs/licensing/CLA.md`. The contributor retains copyright and grants the Steward a perpetual, worldwide, non-exclusive, transferable, sublicensable, royalty-free copyright and patent license to the contribution as part of the Material. Each grant SHALL be conditioned on the Steward's compliance with the outbound commitment.

#### Scenario: A contribution is merged

- **WHEN** a contribution is merged into the Material
- **THEN** the Steward holds the conditional grants above, the contributor keeps every right they had, and the contribution is licensed outbound under the Project License

#### Scenario: A submission marked as not a contribution

- **WHEN** a communication is conspicuously marked "Not a Contribution"
- **THEN** no grant arises from it

### Requirement: The Project License includes its additional permissions

For the agreement, the Project License SHALL mean the license or licenses under which the Material is made available on the Submission Date, together with any additional permissions forming part of their terms, and including any right to adopt a later version where the license so permits. The availability obligation therefore covers the application-store permission.

`[⚖ Confirm this definition cleanly captures the COPYING + COPYING.exceptions bundle.]`

#### Scenario: A future Steward drops the store permission for new code

- **WHEN** the Steward stops granting the application-store permission on code the Steward writes later
- **THEN** every contribution accepted while the permission was part of the Project License remains available with that permission

### Requirement: Every contribution stays available under the Project License

For so long as the Steward exercises any granted right in a contribution, the Steward SHALL license that contribution, as part of the Material, under the Project License. Ceasing to exercise the rights SHALL NOT revoke any license already granted to a third party.

`[⚖ Confirm the continuing-obligation mechanics: "for so long as We exercise" is intended to avoid an eternal-hosting duty after abandonment while guaranteeing concurrent open availability during any commercial exploitation.]`

#### Scenario: Commercial exploitation

- **WHEN** the Steward distributes a build containing a contribution under commercial terms
- **THEN** the same contribution is concurrently available under the Project License

#### Scenario: Project abandonment

- **WHEN** the Steward stops distributing the Material entirely
- **THEN** no duty to keep hosting arises, and every license already granted downstream survives

### Requirement: Relicensing is bounded to open-source-approved terms

In addition to the Project License, the Steward MAY license a contribution, as part of the Material, under any license approved by the Open Source Initiative on or after the Effective Date, whether or not later disapproved, including any later-version right such a license carries. The Steward SHALL NOT license a contribution under any other open terms.

#### Scenario: A move to an OSI-approved license

- **WHEN** the Steward relicenses the Material to an OSI-approved license
- **THEN** every contribution is within the grant, and remains available under the Project License as well

#### Scenario: A move to a non-approved license

- **WHEN** the Steward purports to license a contribution under terms the OSI has never approved and that are not a commercial exception under the next requirement
- **THEN** that licensing is outside the grant and infringes the contributor's copyright

### Requirement: Commercial exceptions are subordinate to open availability

The Steward MAY license a contribution, as part of the Material, under commercial or proprietary terms. That permission SHALL be subordinate to, and its exercise conditioned on, the Steward's continuing compliance with the availability obligation. This permission SHALL NOT be read as implying the application-store permission or vice versa; they are separate doors.

#### Scenario: Selling a commercial license

- **WHEN** the Steward sells a commercial license to a build containing a contribution while the Material stays available under the Project License
- **THEN** the sale is within the grant

#### Scenario: Commercial-only distribution

- **WHEN** the Steward distributes a contribution under commercial terms while no longer offering it under the Project License
- **THEN** the commercial permission is not available, and the grants are outside their condition

### Requirement: Breach gives the contributor remedies

If the Steward fails the outbound commitment, the contributor MAY terminate the agreement by written notice. Termination SHALL NOT affect any license already granted to a third party under the Project License or an approved relicensing. These remedies SHALL be cumulative with those arising from the conditional character of the grants.

`[⚖ Condition-vs-covenant characterization must survive under the chosen governing law so that breach sounds in infringement, not only contract.]`

#### Scenario: A contributor terminates

- **WHEN** a contributor terminates after a breach
- **THEN** downstream licenses already granted remain in force, and the contributor's infringement remedies are unaffected

### Requirement: The patent grant is revocable against offensive assertion

The contributor SHALL have the right to revoke the patent license, prospectively and in writing, if the Steward makes a patent infringement claim targeted at the contribution that is not for a Defensive Purpose — that is, not against a party that has filed, maintained, threatened or joined a patent suit against the Steward or any licensee of the Material.

#### Scenario: A defensive counter-assertion

- **WHEN** the Steward asserts patent claims against a party that has sued a licensee of the Material
- **THEN** the contributor's revocation right does not arise

### Requirement: The Steward is named and has a successor

The agreement SHALL name the Steward and SHALL provide for succession: designation by written declaration, an ordered acceptance chain on death or incapacity without one, and a lapse of the relicensing and commercial permissions if nobody accepts. The Material SHALL record the current Steward and whether a designation is in effect.

`[⚖ The succession chain is load-bearing for a solo-steward project; its window and fallback organisation are blanks for counsel and the Steward.]`

#### Scenario: A successor is designated

- **WHEN** the Steward designates a successor by written declaration, effective now or on a stated condition such as death, incapacity or the formation of a named entity
- **THEN** the successor's written agreement to every right and obligation is recorded with the designation, and the designation may be revoked or replaced until it takes effect

#### Scenario: Death without designation

- **WHEN** the Steward dies or is incapacitated with no effective designation
- **THEN** the rights and obligations pass to the first acceptor in the agreement's ordered chain within its window, and no contributor's downstream license is affected

#### Scenario: Nobody accepts

- **WHEN** no successor accepts within the window
- **THEN** the relicensing and commercial permissions lapse for everyone, and every license already granted under the Project License survives because each recipient holds it from the original licensors directly

#### Scenario: A contributor looks up the Steward

- **WHEN** a contributor opens `docs/licensing/STEWARD.md`
- **THEN** they find the current Steward, the governing law, and whether a succession designation is in effect and since when, without its private terms

### Requirement: Material the contributor does not own is registered or refused

A contribution MAY carry material the contributor does not own only if each such portion is identified in the pull request and at its site, is under a license on the compatible allowlist, keeps its own notice, and is recorded in the third-party registry. Material with no acceptable license, or no human authorship the contributor can warrant, SHALL NOT be accepted as a Contribution.

#### Scenario: A ported routine

- **WHEN** a pull request ports a routine from an MIT-licensed project, says so in the description and in a comment at the site, adds the holder's copyright line, and registers the portion
- **THEN** it is accepted, and the containing file's own license is unchanged

#### Scenario: A snippet with no stated license

- **WHEN** a pull request includes code found without a license statement
- **THEN** it is refused however small the portion, because an unlicensed work grants nothing

#### Scenario: A file copied wholesale

- **WHEN** a pull request adds a whole file from another project
- **THEN** the file keeps its own license identifier and text, is registered as third-party, and is exempt from the header check on that basis

#### Scenario: Employer-owned code

- **WHEN** a contribution contains code the contributor's employer owns
- **THEN** it needs the entity agreement signed by the employer or a written waiver from the employer attached to the pull request

#### Scenario: Output without creative direction

- **WHEN** a contribution consists of material an AI system produced without the contributor's creative direction, review and editing
- **THEN** it is not something the contributor can warrant owning and is not a Contribution; assisted work under the contributor's direction and review is, and on doubt the contributor describes the process and the Steward decides

### Requirement: The pledge quotes the agreement

The licensing overview at `docs/licensing/README.md` SHALL carry the public pledge by quoting the agreement's outbound commitment verbatim rather than paraphrasing a promise beside it, and SHALL state that the Steward's own future code is not bound by the agreement. `CONTRIBUTING.md` SHALL link to it rather than restate it.

#### Scenario: A reader checks the pledge against the agreement

- **WHEN** a reader compares the pledge in the licensing overview with the agreement's outbound commitment
- **THEN** the quoted text is identical

#### Scenario: A contributor starts from CONTRIBUTING

- **WHEN** a contributor reads `CONTRIBUTING.md`
- **THEN** its licensing section is one or two paragraphs naming the license and the agreement and linking to the license texts, the additional permission, the agreement and the overview, with no further licensing prose of its own

### Requirement: The enforcement-intent statement is published

The licensing overview at `docs/licensing/README.md` SHALL carry an enforcement-intent statement in the Steward's voice. It SHALL state that self-hosting is always fine, that hosting unmodified builds is fine, that forks are allowed and rebrand, that there is no proprietary core behind an open wrapper, that the license is enforced for its purpose — a modified hosted Station offers its source — and that the commercial license is the alternative for those who want otherwise.

#### Scenario: A would-be host reads the statement

- **WHEN** someone considering hosting Waypoints reads the statement
- **THEN** they can tell, without legal advice, that unmodified hosting and self-hosting carry no obligation beyond the license's ordinary terms and that modifying a hosted Station obliges them to offer the modified source

### Requirement: Every contribution lands with recorded assent to a CLA version

No contribution SHALL be merged unless the contributor's assent to the agreement, identified by its version, is recorded and verifiable from the pull request. The agreement SHALL carry a version, and a text change SHALL bump it. The text signed SHALL be the repository's `docs/licensing/CLA.md`, published to the assent service from the repository on each bump and never edited elsewhere.

`[⚖ Confirm assent-mechanism validity and record-keeping requirements under Connecticut law.]`

#### Scenario: A first-time contributor

- **WHEN** a pull request arrives from an author with no recorded assent
- **THEN** the pull request's status check stays pending and a comment tells the author where to sign; the check passes once they have

#### Scenario: The agreement changes

- **WHEN** the agreement's text changes and its version is bumped
- **THEN** the published copy is updated from the repository, and a contributor whose recorded assent names an earlier version is asked to assent again before their next contribution merges

#### Scenario: A returning contributor

- **WHEN** a pull request arrives from an author whose recorded assent names the current version
- **THEN** nothing further is asked of them

#### Scenario: A text change without a version bump

- **WHEN** `docs/licensing/CLA.md` differs from the published copy but its version line does not
- **THEN** the licensing check fails naming the file, and the published copy is not updated

#### Scenario: The published copy drifts

- **WHEN** the published copy is edited anywhere other than the repository
- **THEN** the licensing check fails, and the next publication overwrites it from the repository

#### Scenario: A draft that cannot be published yet

- **WHEN** `docs/licensing/CLA.md` still holds a bracketed blank
- **THEN** the licensing check reports it as an unpublished draft and passes, and publication is refused until the blank is filled

#### Scenario: What a signer is asked

- **WHEN** a contributor signs
- **THEN** they are asked for their full name and email, to confirm they are at least eighteen years old, and to confirm they have read and agree; no other personal data is collected, and no option to sign on an employer's behalf is offered until the entity agreement exists
