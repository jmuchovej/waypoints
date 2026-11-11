# licensing/source-notices Specification

## Purpose

Defines the license notice every first-party source file carries — the full form at entry points, SPDX tags in the initial comment block everywhere else — which files are header-bearing, which are exempt and why, how the copyright year is fixed, and the check that keeps header, manifest and policy in agreement on every pull request.

## Requirements

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

The header check SHALL apply to first-party files with these extensions: `rs`, `ts`, `vue`, `js`, `mjs`, `css`, `toml`, `nix`, `typ`, `nu`, `sh`, plus the extensionless scripts under `tooling/bin/`. It SHALL NOT apply to lockfiles, `.d.ts` declarations, data and asset formats, platform project files under `src-tauri/gen/`, configuration devenv writes on shell entry, or third-party code in the tree. Exemption SHALL be by path, never by weakening the check.

#### Scenario: A first-party source file without a header

- **WHEN** a header-bearing file has no `SPDX-License-Identifier` line in its initial comment block
- **THEN** the header check fails and names the file

#### Scenario: Third-party code in the tree

- **WHEN** a file under `tooling/gh-workflow/` or `tooling/patches/` is examined by the header check
- **THEN** it is skipped: that code carries its upstream's license and attribution, not ours

#### Scenario: A platform project file

- **WHEN** a file under `src-tauri/gen/` is examined
- **THEN** it is skipped as platform scaffolding

#### Scenario: Configuration devenv generates

- **WHEN** a mise fragment under `.config/mise/conf.d/`, which devenv rewrites on shell entry, is examined
- **THEN** it is skipped: a tag added by hand would fail the generated-file lockstep check, and the fragment is configuration, not source

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

## Decisions

### 2026-11-02 — SPDX headers everywhere now; REUSE deferred

**Decision.** Every first-party source file carries an SPDX header; the project does not adopt the REUSE specification (no `LICENSES/` directory, no `REUSE.toml`), and a first-party check stands in for `reuse lint`.

**Rationale.** REUSE's value is machine-checking a mixed-license map. At a uniform policy that value is nil, and SPDX headers alone give scanners what they need. The headers are already REUSE-formatted, so adopting it later is additive — a `LICENSES/` directory and a root `REUSE.toml`, both spec-mandated locations — not a rewrite. The first-party check also verifies something `reuse lint` does not: that each header agrees with its owning manifest and that manifests agree with the policy.

**Rejected alternatives.**

- _Full REUSE now._ Two more root entries for a check that passes vacuously on a uniform tree.
- _REUSE with `.license` sidecars for header-less files._ The same root cost without the config file; only worth it once a second tier exists.

### 2026-11-02 — The "or later" election lives in the notices

**Decision.** The license texts in the tree are the version 3 text; the grant of "version 3 or (at your option) any later version" is made by the per-file notices and the `-or-later` SPDX identifier.

**Rationale.** The FSF permits verbatim copying of the license text only. Applying it means vendoring the text, writing notices that invoke it, and writing the additional permission as its own file that the notices reference; the license of the Program is the union of the text and the permission, assembled by the notices.

### 2026-11-02 — Header scope by file type; exemption by path

**Decision.** Header-bearing types are `rs`, `ts`, `vue`, `js`, `mjs`, `css`, `toml`, `nix`, `typ`, `nu`, `sh` and the extensionless scripts under `tooling/bin/`. Data, configuration and document formats (`yaml`, `json`, `xml`, `md`, Kotlin and Swift under the platform projects, OpenTofu, images, fonts, lockfiles, `.d.ts`) carry none. Registered third-party code and platform scaffolding are exempt by path.

**Rationale.** The line is "code the application or its tooling is built from" against "data and configuration". JSON cannot carry a comment; the fifty-odd YAML files are nearly all generated workflows whose first line is already a generator marker and whose linters expect a known shape. A header asserts terms: where the terms are not ours (third-party code) the file carries none, and the check exempts it rather than failing on it. Generators emit the header so regeneration never strips it.

**Rejected alternatives.**

- _Headers in YAML and JSON via REUSE sidecars._ That is REUSE adoption, deferred above.
- _Failing on exempt directories until they are "fixed"._ Third-party code will never be fixed in that sense; the exemption is the correct steady state.

### 2026-11-04 — Two tags in the initial comment block; the year is first authorship and never changes

**Decision.** A header-bearing file's initial comment block carries at least one `SPDX-FileCopyrightText: <year> <holder>` line and exactly one `SPDX-License-Identifier: AGPL-3.0-or-later` line, after any shebang or tool directive the first line requires. The year is the single year the file was first authored and is never updated; the check requires a four-digit year and does not compare it with the calendar. The holder is John Muchovej; contributors who hold copyright, and the upstream holders of registered ported portions, get their own line. A pointer to the additional permission may follow and is not required.

**Rationale.** "The tags are present and agree" is the whole invariant; a fixed three-line form within a fixed window adds brittleness against directives and multi-holder files without adding meaning. The copyright year records first publication, so updating it yearly is both churn and a misstatement, and a range misstates it twice. The additional permission is a term of the license the identifier names and is spelled out in the full notice at every entry point, so repeating the pointer in every file buys nothing.

**Rejected alternatives.**

- _A mandatory three-line header within the first five lines._ Brittle, and wrong for a file with two holders.
- _Updating the year on edit, or a `first-last` range._ Annual churn across the tree for no legal gain.
- _Requiring the Steward as the only holder._ Contradicts the agreement, under which contributors retain copyright.
