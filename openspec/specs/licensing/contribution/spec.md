# licensing/contribution Specification

## Purpose

States what a contributor grants and what they are owed in return: the contributor license agreement's load-bearing clauses as invariants, the public pledge that quotes them, the enforcement-intent statement, and the rule that every contribution lands with recorded assent to a specific version of the agreement.

## Requirements

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

`[⚖ Confirm this definition cleanly captures the LICENSE + COPYING.exceptions bundle.]`

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

### Requirement: The Steward signs as a contributor

The individual who is Steward SHALL sign the agreement in their individual capacity, as a contributor, with the Steward as the other party, covering the work they authored before signing. Theirs SHALL be the portal's first record, and `docs/licensing/STEWARD.md` SHALL say whether it exists. They keep ownership of their work, as any contributor does.

`[⚖ One person on both sides may form no contract. Confirm it reaches successors and earlier work.]`

#### Scenario: The Steward transfers to an incorporated entity

- **WHEN** a Legal Entity succeeds as Steward
- **THEN** the former Steward's work is already within the grant, and they sign nothing new

#### Scenario: The Steward dies with a successor named by the estate

- **WHEN** a successor accepts under the agreement's chain
- **THEN** the former Steward's work is within the grant to the same extent as any contributor's

#### Scenario: While both roles are one person

- **WHEN** the Steward licenses their own work while they remain Steward
- **THEN** the signature restricts nothing they could otherwise do with it, because the grant is non-exclusive and they keep ownership

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
- **THEN** its licensing section is one or two paragraphs naming the license and the agreement, saying whether outside contributions are open, and linking to the license texts, the additional permission, the agreement and the overview, with no further licensing prose of its own

### Requirement: The enforcement-intent statement is published

The licensing overview at `docs/licensing/README.md` SHALL carry an enforcement-intent statement in the Steward's voice. It SHALL state that self-hosting is always fine, that hosting unmodified builds is fine, that forks are allowed and rebrand, that there is no proprietary core behind an open wrapper, that the license is enforced for its purpose — a modified hosted Station offers its source — and that the commercial license is the alternative for those who want otherwise.

#### Scenario: A would-be host reads the statement

- **WHEN** someone considering hosting Waypoints reads the statement
- **THEN** they can tell, without legal advice, that unmodified hosting and self-hosting carry no obligation beyond the license's ordinary terms and that modifying a hosted Station obliges them to offer the modified source

### Requirement: External contributions are closed until the CLA portal is live

Contributions from anyone other than the Steward SHALL NOT be merged until the project's own CLA portal is deployed on the live website and records assent. `CONTRIBUTING.md` and the licensing overview SHALL say so. No third-party assent service SHALL stand in for the portal: the agreement is a legal record, and CLA assistant proved too unreliable to hold one.

#### Scenario: A pull request from an outside contributor

- **WHEN** someone other than the Steward opens a pull request while the portal is not live
- **THEN** it is not merged, whatever its merit, and the contributing guide has already told them why and invited an issue instead

#### Scenario: The Steward's own work

- **WHEN** the Steward opens a pull request while the portal is not live
- **THEN** the restriction does not apply

#### Scenario: The portal goes live

- **WHEN** the portal is deployed and records assent
- **THEN** the change that opens contributions updates `CONTRIBUTING.md`, the overview and this requirement together

### Requirement: Every contribution lands with recorded assent to a CLA version

Once the portal is live, no contribution SHALL be merged unless the contributor's assent to the agreement is recorded and verifiable from the pull request. The agreement SHALL carry a version, and a text change SHALL bump it. The text signed SHALL be the repository's `docs/licensing/CLA.md`, and each record SHALL identify it by version and by a hash of the text.

`[⚖ Confirm assent-mechanism validity and record-keeping requirements under Connecticut law.]`

#### Scenario: A first-time contributor

- **WHEN** a pull request arrives from an author with no recorded assent
- **THEN** the pull request's status check stays pending and tells the author where to sign; the check passes once they have

#### Scenario: The agreement changes

- **WHEN** the agreement's text changes and its version is bumped
- **THEN** a contributor whose recorded assent names an earlier version is asked to assent again before their next contribution merges

#### Scenario: A returning contributor

- **WHEN** a pull request arrives from an author whose recorded assent names the current version
- **THEN** nothing further is asked of them

#### Scenario: A draft with blanks

- **WHEN** `docs/licensing/CLA.md` still holds a bracketed blank
- **THEN** it is not offered for signing

#### Scenario: What a signer is asked

- **WHEN** a contributor signs
- **THEN** they are asked for their full name and email, to confirm they are at least eighteen years old, and to confirm they have read and agree; no other personal data is collected, and no option to sign on an employer's behalf is offered until the entity agreement exists

## Decisions

### 2026-08-28 — A contributor license agreement, not a Developer Certificate of Origin

**Decision.** Contributions are accepted under a CLA derived from the Harmony Individual Contributor License Agreement (HA-CLA-I) version 1.0, with its outbound clause rewritten.

**Rationale.** Dual licensing — selling commercial exceptions while the Material stays open — requires relicensing rights that a DCO cannot give: a DCO certifies provenance under the inbound license and grants nothing further. Harmony is a well-understood base with a CC-BY-3.0 text that may be adapted.

**Rejected alternatives.**

- _DCO only._ No relicensing right; forecloses commercial exceptions and any future move to a later or different open license.
- _Copyright assignment._ Takes more from contributors than the project needs and is unenforceable in some jurisdictions.

### 2026-08-31 — An asymmetric outbound clause, with the grants conditioned on it

**Decision.** Harmony's five mutually exclusive outbound options are replaced by an obligation plus two bounded permissions: perpetual availability under the Project License (a), relicensing only to OSI-approved terms (b), commercial exceptions subordinate to (a) (c), and an express condition (d) that the copyright and patent grants depend on compliance with all three. A contributor may terminate for breach; downstream licenses survive; remedies are cumulative.

**Rationale.** CLA plus AGPL is the power structure that lets a steward sell exceptions, and also the one that let other projects flip to source-available terms. Making the pledge a _condition_ of the grant rather than a covenant means a flip outside (a)–(c) is infringement, not mere breach — the strongest guarantee a solo steward can offer without giving up dual licensing. The Project License is defined to include additional permissions so that contributed code can never lose application-store distributability even if a future Steward stops offering the permission on new code. The Steward's own future code is not bound; the pledge and the enforcement-intent statement speak for that in the Steward's voice.

**Rejected alternatives.**

- _Harmony's option "any license approved by OSI" alone._ Permits dropping the Project License entirely.
- _A covenant not to relicense restrictively._ Breach sounds in contract only; the remedy is damages, not the grant falling away.
- _Promising the pledge beside the agreement rather than quoting it._ Two texts drift; one is the operative one.

### 2026-11-05 — CLA assistant dropped as unreliable; external contributions closed until a portal of our own records assent

**Decision.** Assent will be recorded by a CLA portal on the live website, delivered by a follow-on change. Until it is live, only the Steward's work merges, and `CONTRIBUTING.md` and the licensing overview say so. Signers will give full name and email, confirm they are at least eighteen, and confirm agreement; nothing else, and no employer option until the entity agreement exists.

**Rationale.** The license structure needs a signed grant: a Developer Certificate of Origin certifies provenance and cannot give the relicensing and commercial rights. This change first used CLA assistant for that, because it was already configured and needed no code of ours. It was dropped because it proved unreliable. On 2026-11-05 its sign page offered nothing to sign; the call supplying the agreement took a minute for this repository and timed out or returned a 500 for others; and open upstream reports described blank pages, checks stuck daily, and one organisation's contributor counts reading zero. A legal record cannot rest on that, and with no signatures collected, leaving cost nothing. The website already has GitHub sign-in, a database and a Worker, so a portal is a small build rather than a new service. Closing contributions until then is honest about the gap: merging first and collecting signatures later would rely on a grant nobody had made. The fields follow §3 of the agreement and nothing more.

**Rejected alternatives.**

- _Staying on CLA assistant, re-linked or through its GitHub App._ The failures reached other projects, so they were not this repository's credentials.
- _`contributor-assistant/github-action`._ Archived, last released in September 2024.
- _Signing by pull request comment, with a signature file on a branch._ No form, so none of what §3 relies on, and a public file should not hold personal data.
- _A Developer Certificate of Origin alone._ Grants nothing past the inbound license.
- _Accepting contributions meanwhile and collecting signatures later._ Every merge would rest on a grant nobody had made.

### 2026-11-05 — The Steward signs as a contributor, in an individual capacity

**Decision.** The individual who is Steward signs the agreement as a contributor, with the Steward as the other party, as the portal's first record and covering work authored before signing. `docs/licensing/STEWARD.md` records whether that signature exists.

**Rationale.** While one person holds both roles, naming two does not make two legal persons, so the signature probably forms no contract today and restricts nothing: the grant is non-exclusive and the author keeps ownership. It matters at succession. §7.3 passes the rights received through the agreement, and the founder's code is received through it only if the founder signed. Otherwise a successor, whether an incorporated entity or one named by an estate, would hold every contributor's grant and none for the largest author, and could not exercise the relicensing or commercial permissions over the Material as a whole. Signing once now spares a new signature at incorporation, subject to counsel on the open points.

**Rejected alternatives.**

- _Not signing, since the Steward cannot contract with themselves._ True today and wrong at succession, and on death there may be nobody left to sign.
- _Leaving it to an assignment at incorporation._ Covers incorporation only, and an incorporation would likely add one regardless.

### 2026-11-04 — Licensing prose lives in the overview; the contributor guide defers to it

**Decision.** The long-form licensing prose — the bundle, the enforcement-intent statement, the pledge quoting the agreement, the header rule, the non-owner section, the plugin-boundary note, that contributions are closed until the portal is live — lives in `docs/licensing/README.md`. `CONTRIBUTING.md` carries one or two paragraphs and links to the license texts, the permission, the agreement, the Steward record and the overview.

**Rationale.** `docs/` is what the website publishes, so the overview is written once where it is read most. A contributor guide that restates licensing drifts from the texts it paraphrases; a guide that links cannot.

**Rejected alternatives.**

- _`CONTRIBUTING.md` absorbs everything, for a flat root._ The guide becomes a second licensing document to keep in step with the first.

### 2026-11-04 — Succession is an ordered chain that ends in lapse, and the Steward is a public record

**Decision.** The agreement's §7.3 is restructured: assignment with written acceptance (a); designation of a successor by written declaration, effective now or on a stated condition such as death, Incapacity or the formation of a named entity, revocable until it takes effect (b); for an individual Steward who dies or suffers Incapacity without an effective designation, an ordered chain — a successor named by the estate, then a named fallback organisation, then any free-software stewardship entity the estate names — each accepting in writing within a window, with a dissolution mirror for a Legal Entity Steward (c); lapse of the relicensing and commercial permissions if nobody accepts, every downstream license surviving (d); and continuity without re-assent (e). `docs/licensing/STEWARD.md` records the current Steward and the existence and date of any designation.

**Rationale.** The chain is a residuary clause in the shape of a will: each tier is tried in order and only the first acceptor takes. The lapse is safe for everyone because of how the AGPL is built — under its section 10 each recipient of the Material is licensed directly by the original licensors, every contributor and the Steward, not by "Us" as an intermediary — so when the Steward disappears without a successor, nobody's existing freedom to use, modify or redistribute depends on a licensor who no longer exists; what disappears is only the dual-licensing optionality in §2.3(b)–(c), the power the agreement was careful to bound. The Legal-Entity mirror is the FLA 2.0 idea and lets the clause survive the Steward's own incorporation without a rewrite. Conditional-effect designation is the operational point: a one-page declaration naming a person "upon my death or Incapacity" can be signed now, and a later one naming "Waypoints, Inc. upon its formation" is the whole legal half of incorporating. The public record lets contributors see that a line of succession exists without publishing its terms.

**Rejected alternatives.**

- _A single blank for "the mechanism"._ Load-bearing for a solo steward and unfillable without a structure to fill it into.
- _Passing the Steward's rights to the estate indefinitely._ An estate is not a steward; the chain gives it a window to name one and then closes the dual-licensing door rather than leaving it ajar.
- _Naming only a generic fallback._ A specific organisation is better insurance if it will accept and worthless if it will not; the clause names one as a bracketed blank until it has been asked.

### 2026-11-04 — Material the contributor does not own is registered or refused; undirected generation is not a Contribution

**Decision.** A contribution may carry material the contributor does not own only if each portion is identified in the pull request and at its site, is under a license on the compatible allowlist, keeps its own notice, and is recorded in the third-party registry. Code the employer owns needs the entity agreement or a written waiver. Material with no stated license is refused however small. Output an AI system produced without the contributor's creative direction, review and editing is not something the contributor can warrant owning and is not a Contribution; assisted work under direction and review is, and on doubt the contributor describes the process and the Steward decides. The agreement's non-owner pointer resolves to this section of the licensing overview, as in force on the Submission Date.

**Rationale.** The agreement asks contributors to own what they submit; real contributions sometimes carry a ported routine or a specification fixture, and the honest answer is a path for it rather than a prohibition nobody follows. Every rule maps to an invariant the tree already holds: one allowlist, attribution beside the code, the registry feeding the notices, the file's own identifier unchanged. The authorship rule follows from copyright protecting human authorship: the contributor can warrant only what they hold, and the line between assisted and generated is fuzzy enough that a disclosure-on-doubt rule with the Steward deciding is the only version that can be applied consistently; a disclosure gate on every pull request would be unenforceable. The pointer rather than inline text because the instructions name repository machinery that will evolve, and git plus the Submission Date pin what a contributor accepted.

**Rejected alternatives.**

- _Forbidding third-party material in contributions._ Unrealistic, and it would push the material in undeclared.
- _Inlining the instructions in the agreement._ Freezes repository mechanics into a legal text that should change rarely.
- _A mandatory AI-use disclosure gate._ Modelled on a registration requirement, not a merge condition; unenforceable at the fuzzy boundary.
