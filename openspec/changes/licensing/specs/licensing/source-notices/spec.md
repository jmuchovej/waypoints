# Spec Delta

## Purpose

Defines the license notice every first-party source file carries — the full form at entry points, SPDX tags in the initial comment block everywhere else — which files are header-bearing, which are exempt and why, how the copyright year is fixed, and the check that keeps header, manifest and policy in agreement on every pull request.

## ADDED Requirements

### Requirement: Entry points carry the full notice

Each first-party crate's library root, each binary's main source file, the frontend application entry, and the README SHALL carry the full license notice: the SPDX copyright and license tags, the GNU appendix notice electing version 3 or any later version, the warranty disclaimer, the pointer to `https://www.gnu.org/licenses/`, and one paragraph stating that an additional permission under section 7 is granted on the terms in `docs/licensing/COPYING.exceptions`.

#### Scenario: A crate root

- **WHEN** a first-party crate's `lib.rs` is read
- **THEN** its leading comment is the full notice in the form below, with the year of the file's first authorship

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

#### Scenario: The README

- **WHEN** `docs/README.md` is read
- **THEN** it has a license section stating the license, electing any later version, naming the root `LICENSE`, `docs/licensing/LICENSE.md`, `docs/licensing/COPYING.exceptions`, `docs/licensing/CLA.md`, `docs/licensing/README.md` and `docs/CONTRIBUTING.md`, and stating that the additional permission applies

### Requirement: Every other header-bearing file carries SPDX tags in its initial comment block

Every first-party source file that is not an entry point SHALL open with a comment block — after any shebang or tool directive the file type requires on its first line — containing at least one `SPDX-FileCopyrightText: <year> <holder>` line and exactly one `SPDX-License-Identifier: AGPL-3.0-or-later` line, in the file type's own comment syntax. A line pointing at `docs/licensing/COPYING.exceptions` MAY follow; nothing else is required of the block.

#### Scenario: A Rust, TypeScript, Vue, JavaScript or CSS file

- **WHEN** a first-party file of one of these types is read
- **THEN** its initial comment block, in that language's line-comment or block-comment syntax, contains both tags

#### Scenario: A TOML or Nix file

- **WHEN** a first-party `.toml` or `.nix` file is read
- **THEN** both tags appear as `#` comments, after any `#:tombi` directive the file begins with

#### Scenario: A script with a shebang

- **WHEN** a script under `tooling/bin/` or a `.nu` file is read
- **THEN** the shebang is the first line and the comment block carrying both tags follows it

#### Scenario: A Typst source

- **WHEN** a first-party `.typ` file is read
- **THEN** both tags appear as `//` comments

#### Scenario: A second copyright holder

- **WHEN** a file's initial comment block carries two `SPDX-FileCopyrightText` lines with different holders or years
- **THEN** it conforms; contributors retain their copyright and may add their own line

### Requirement: The copyright year is the year of first authorship

The year in a file's `SPDX-FileCopyrightText` line SHALL be the single four-digit year in which that file was first authored, and SHALL NOT change afterwards. The check SHALL require a four-digit year and SHALL NOT compare it with the current date.

#### Scenario: A file is edited years later

- **WHEN** a file first authored in 2026 is modified in 2029
- **THEN** its copyright line still reads 2026 and the check passes

#### Scenario: A new file

- **WHEN** a first-party source file is created
- **THEN** its copyright line carries the year of creation

#### Scenario: A year range

- **WHEN** a copyright line reads `2026-2029`
- **THEN** the check fails naming the file, because the year is a single year of first authorship

### Requirement: Header-bearing files are defined by type and ownership

The header check SHALL apply to first-party files with these extensions: `rs`, `ts`, `vue`, `js`, `mjs`, `css`, `toml`, `nix`, `typ`, `nu`, `sh`, plus the extensionless scripts under `tooling/bin/`. It SHALL NOT apply to lockfiles, `.d.ts` declarations, data and asset formats, platform project files under `src-tauri/gen/`, third-party code in the tree, or directories whose license is not yet decided. Exemption SHALL be by path, never by weakening the check.

#### Scenario: A first-party source file without a header

- **WHEN** a header-bearing file has no `SPDX-License-Identifier` line in its initial comment block
- **THEN** the header check fails and names the file

#### Scenario: Third-party code in the tree

- **WHEN** a file under `tooling/gh-workflow/` or `tooling/patches/` is examined by the header check
- **THEN** it is skipped: that code carries its upstream's license and attribution, not ours

#### Scenario: A platform project file

- **WHEN** a file under `src-tauri/gen/` is examined
- **THEN** it is skipped as platform scaffolding

#### Scenario: A pending surface

- **WHEN** a file under a directory listed as awaiting the Steward's license decision is examined
- **THEN** it is skipped, and the directory remains listed until decided

### Requirement: Generated files get their header from the generator

A first-party generator that writes source files into the tree SHALL emit the SPDX tags as part of its output, so a regenerated file passes the header check without a separate step. When the output file already exists, the generator SHALL keep its existing copyright year; for a new output file it SHALL use the current year. The header check SHALL treat generated files like any other first-party file of their type.

#### Scenario: Bindings are regenerated

- **WHEN** the TypeScript bindings are regenerated from the Rust data model
- **THEN** each output file begins with its generated-file marker and both tags, with the year it already carried, and the header check passes on it

#### Scenario: A generated file is hand-edited to remove the header

- **WHEN** a generated file's header is removed by hand
- **THEN** the header check fails on it, and the generated-file lockstep check fails too because regeneration restores it

### Requirement: Header, manifest and policy agree

For every header-bearing file, the SPDX expression in its `SPDX-License-Identifier` line SHALL equal the license expression of the workspace crate whose directory contains it, or the workspace policy for a file outside any crate. Every first-party manifest's expression SHALL be a valid, non-deprecated SPDX expression satisfied by the policy. Both SHALL be checked on every pull request and before every push.

#### Scenario: A header disagrees with its crate

- **WHEN** a file under a crate declaring `AGPL-3.0-or-later` carries `SPDX-License-Identifier: MIT`
- **THEN** the check fails naming the file, the header expression and the manifest expression

#### Scenario: A manifest uses a deprecated identifier

- **WHEN** a first-party manifest declares `license = "AGPL-3.0"`
- **THEN** the check fails naming the crate and the canonical replacement, because the bare identifier is deprecated on the SPDX list

#### Scenario: A header with an invalid expression

- **WHEN** a header's identifier does not parse as an SPDX expression
- **THEN** the check fails naming the file and the parse error

#### Scenario: A placeholder holder

- **WHEN** a copyright line still contains a bracketed placeholder
- **THEN** the check fails naming the file

#### Scenario: Everything agrees

- **WHEN** every first-party manifest equals the policy and every header equals its owning manifest
- **THEN** the check passes and reports the number of crates and the SPDX list version it judged against

### Requirement: Contributors are told the header rule

The licensing overview at `docs/licensing/README.md` SHALL state the header rule — which files carry it, both forms, the year rule, and that the check fails a pull request without it — so the rule is policy whether or not a tool enforces it. `CONTRIBUTING.md` SHALL say that every source file carries the header and link to the overview for the text.

#### Scenario: A contributor adds a file

- **WHEN** a contributor follows the link from `docs/CONTRIBUTING.md` before adding a source file
- **THEN** they find the exact tags to paste, the year rule, and the list of file types it applies to
