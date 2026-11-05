# Licensing

Waypoints is licensed under the GNU Affero General Public License, version 3 or (at your option) any later version, together with an additional permission under its section 7 for conveyance through application stores. This page explains that bundle, what the project promises contributors, and what a contribution has to carry.

## The license

The license is one bundle, in three files:

- [`LICENSE`](../../LICENSE) at the repository root is the Free Software Foundation's plain-text AGPLv3, verbatim.
- [`LICENSE.md`](LICENSE.md) is the Free Software Foundation's own Markdown edition of the same text, verbatim.
- [`COPYING.exceptions`](COPYING.exceptions) holds the additional permission.

The "or any later version" election is made by the notices in the source, not by either edition of the text, because the text may only be copied unmodified.

The additional permission lets anyone convey Waypoints through a third-party store whose non-negotiable terms conflict with the license. It has two conditions: comply with everything the store does not prevent, and keep the same version's Corresponding Source available under the plain license through a channel the store does not control. It runs to every recipient, forks included. You may remove it from copies you convey.

Everything first-party in this repository carries these terms: the Rust crates, the frontend, the website's code, and the developer tooling. Non-code assets bundled into the application carry them too. Third-party code in the tree keeps its own license and is listed in [`.config/third-party.toml`](../../.config/third-party.toml).

The dependencies' licenses are listed in `THIRD-PARTY-NOTICES.md`, which ships with the application.

## How we enforce it

<!-- The Steward's statement goes here. Until it is written, these are the points it must make. -->

The statement of enforcement intent, in the Steward's voice, is still to be written. It will say:

- Self-hosting is always fine.
- Hosting unmodified builds is fine.
- Forks are allowed, and rebrand.
- There is no proprietary core behind an open wrapper.
- The license is enforced for its purpose: a modified hosted Station offers its source.
- A commercial license is the alternative for those who want otherwise.

## What we promise contributors

Contributions will be accepted under the [Waypoints Individual Contributor License Agreement](CLA.md). The promise below is section 2.3 of that agreement, quoted rather than paraphrased, because the agreement is what binds:

> (a) **Availability obligation.** For so long as We exercise any of the rights granted in Sections 2.1 and 2.2 with respect to Your Contribution, We agree to license the Contribution, as part of the Material, under the terms of the Project License. For the avoidance of doubt: (i) additional permissions forming part of the Project License (including any application store permission) are within this obligation; and (ii) Our ceasing to exercise the granted rights does not revoke any license already granted to any third party.
>
> (b) **Open relicensing permission.** In addition, We may license the Contribution, as part of the Material, under the terms of any license approved by the Open Source Initiative on or after the Effective Date, whether or not such license is subsequently disapproved, including any right to adopt any later version of such a license where its terms so permit.
>
> (c) **Commercial exception.** In addition, We may license the Contribution, as part of the Material, under other terms selected by Us, including commercial or proprietary terms. This permission is subordinate to, and its exercise is conditioned upon Our continuing compliance with, Section 2.3(a).
>
> (d) **Condition.** The licenses granted in Sections 2.1 and 2.2 are conditioned upon Our compliance with this Section 2.3. Any licensing of the Contribution outside the scope of Sections 2.3(a)–(c) is outside the scope of the rights granted to Us under this Agreement.

This binds the Steward's handling of contributions. It does not bind code the Steward writes in the future, which is the Steward's own to license.

The current Steward, and whether a successor has been designated, is recorded in [`STEWARD.md`](STEWARD.md).

## Contributions are closed for now

Pull requests from outside the project can't be merged yet. The agreement above has to be signed before a contribution is accepted, and the portal for signing it isn't live, so a pull request opened today has no way to be accepted. Please open an issue instead: bug reports and ideas are welcome.

When the portal opens, you will sign in with GitHub, give your name and email, confirm you are at least eighteen, and confirm you agree. The text you sign will be [`CLA.md`](CLA.md) in this repository, which is still a draft. Once you have signed, nothing further is asked of you until the agreement's text changes and its version is bumped.

## Source file headers

Every first-party source file opens with a comment block, after any shebang or tool directive its first line requires, containing at least one copyright line and exactly one license line:

```rust
// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
```

Use the file type's own comment syntax. In Vue files that is an HTML comment, in CSS a block comment, and in TOML, Nix, Nushell and shell scripts `#`.

- **Which files.** `rs`, `ts`, `vue`, `js`, `mjs`, `css`, `toml`, `nix`, `typ`, `nu`, `sh`, and the extensionless scripts under `tooling/bin/`. Data and documents (`yaml`, `json`, `xml`, `md`), `.d.ts` declarations, lockfiles and assets carry none. Neither do `src-tauri/gen/`, `.config/mise/conf.d/`, or code registered as third-party.
- **The year.** It is the single year the file was first authored, and it never changes. A range is rejected.
- **Your copyright.** You keep the copyright in what you write. If you hold copyright in a file, add your own `SPDX-FileCopyrightText` line beside the existing one.

Crate roots, each binary's main file and the frontend's entry point carry the full notice instead:

```rust
// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.
```

`just license` checks all of this, and a pull request without a header fails.

## Contributions containing material you do not own

The contributor agreement asks that you own the copyright in what you submit (CLA §3(b)). Sometimes a contribution has to carry something you do not own: a routine ported from another project, a fixture copied from a specification, a glyph from a licensed icon set, a snippet your employer owns. That is allowed when every such piece can be identified, is under terms this tree already accepts, and keeps its own notice. Concretely:

1. **Say so, in the pull request and at the site.** The pull request description lists each third-party portion: what it is, where it came from (URL and commit or version), who holds the copyright, and its license. In the file, the portion is bracketed by a comment saying the same — `Portions derived from <project> (<URL>, <commit>), Copyright (c) <year> <holder>, licensed <SPDX-ID>` — and the file gains an `SPDX-FileCopyrightText` line for that holder beneath the Steward's.

2. **The license must already be on the allowlist.** The identifiers `.config/deny.toml` accepts for dependencies are the identifiers accepted for in-tree third-party code. There is one list. Anything else — including "no license stated", which is the common case for snippets found in the wild — is not acceptable, however small the portion. If you are unsure, ask in an issue before writing the code.

3. **The notice travels with the code.** For MIT, BSD, ISC and other short licenses, the full license text goes in the comment block beside the portion. For longer texts (Apache-2.0 with its NOTICE, MPL-2.0), the text goes in a `LICENSE-<name>` file in the owning crate or package directory, the way `tooling/patches/` carries `LICENSE-MIT`. Either way, record the portion in `.config/third-party.toml` so that it appears in `THIRD-PARTY-NOTICES.md` and the license check knows about it.

4. **The file's license does not change.** A file combining a compatible third-party portion with first-party code stays under the project license and its `SPDX-License-Identifier` is unchanged; the portion remains under its own terms, which the notice preserves. A file copied wholesale from elsewhere is different: it keeps its own identifier, is listed as third-party code in `.config/third-party.toml`, and is exempt from the header check for that reason — the `tooling/gh-workflow` pattern.

5. **Modified weak-copyleft files stay under their license.** A file under MPL-2.0 or another file-scoped copyleft that you modify stays under that license, keeps its original header, and notes the modification. The license requires this; we only enforce it.

6. **Code your employer owns** needs either the Entity version of the agreement signed by your employer (a known follow-up — until it exists, ask) or a written waiver from your employer attached to the pull request.

7. **Material with no human author is not yours to contribute.** You grant rights only in what you hold copyright in, and copyright protects human authorship. Output an AI system produced without your creative direction, review and editing is not something you own, so it is not something CLA §3(b) lets you warrant. Using coding assistants under your direction, with your review and your edits, is fine — it is how the Steward works. Wholesale generation submitted unread is not. If you are unsure whether what you did meets the bar, describe your process in the pull request and the Steward will decide.

8. **If none of the above fits, it is not a Contribution.** Say "Not a Contribution" in the pull request (CLA §1) and it will not be merged.

## Programs that talk to Waypoints

<!-- The Steward's note goes here. Until it is drafted, this sentence states the intent. -->

Programs that communicate with Waypoints over a defined interface, the sync protocol today, are not derivative works of Waypoints on that basis, and may carry any license, even though the project encourages the AGPL. This is the Steward's stated interpretation and a statement of non-assertion, not a license term.
