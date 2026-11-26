# distribution/package-channels Specification

## Purpose

Governs the distribution channels through which Waypoints is installed — where each lives in the tree, how a package manifest derives from the release manifest rather than restating it, the registry of which distribution channels each release channel reaches and under what identity, how a package-manager-owned install is launched, and how a flavored package manifest is checked before it leaves the tree. A _release channel_ is stable, preview, or nightly; a _distribution channel_ is a package manager or deployment target; a _package manifest_ is a channel's template, a _flavored package manifest_ its render for one release channel, and the _release manifest_ the `{channel}.json` record the publishing run writes.

## Requirements

### Requirement: Every distribution channel lives under one tree

The repository SHALL hold every distribution channel's files under a single top-level `distribution/` directory, one subdirectory per channel, with the channel's recipes reachable from the repository's task runner under one module. The Nix overlay — its recipes, channel reading, release index, and the recipes that refresh its pinned hashes — SHALL be one such channel.

#### Scenario: A channel is found by name

- **WHEN** a contributor looks for the files behind a package manager (Homebrew, the AUR, winget, Chocolatey, Nix) or the Station's reference deployment (Kubernetes)
- **THEN** each is a directory directly under `distribution/`, named for the channel

#### Scenario: A channel's inner layout is its own

- **WHEN** a channel has more than one form of file (a kustomize base and its overlays; build recipes and a release index)
- **THEN** those forms nest inside the channel's directory in its tooling's idiom, the channel's recipes sit at the channel's root, and no form becomes a top-level channel of its own

#### Scenario: Recipes are one module away

- **WHEN** a contributor lists the task runner's recipes for distribution
- **THEN** every channel that has recipes appears as a submodule of one distribution module, and no channel's recipes live under any other module

### Requirement: A distribution registry names each release channel's distribution channels and identities

The repository SHALL hold one registry at the root of `distribution/` with a table per release channel, each entry naming a distribution channel and the package identity that release channel takes there, following the package manager's convention for variants of one application. A release channel absent from a distribution channel's entries SHALL NOT be rendered for it. Identities SHALL keep release channels co-installable wherever the package manager installs side by side.

#### Scenario: The registry decides eligibility

- **WHEN** a render is requested for a release channel the registry does not list under that distribution channel
- **THEN** rendering fails naming the release channels the registry does list for it

#### Scenario: Identities follow the channel's convention

- **WHEN** a distribution channel's package manager has a naming convention for variants of one application
- **THEN** the registry's identities follow it (a versioned-cask suffix, a `-bin` suffix, a dotted identifier segment), and the package manifests read them from the registry rather than spelling them out

#### Scenario: Release channels are distinct packages

- **WHEN** a distribution channel lists more than one release channel
- **THEN** each renders to a distinct package identity, and installing two does not conflict on executable name, application name, or install location

#### Scenario: The registry is the single list

- **WHEN** a release channel is added to or removed from a distribution channel
- **THEN** only the registry changes, and renders for a removed release channel are refused from then on

### Requirement: Package manifests are templates, derived from the release manifest

A package channel's directory SHALL hold package manifests, never a flavored one: outside the release index, no committed file under `distribution/` names a published version, digest, or versioned download URL. Every release-specific value in a flavored package manifest SHALL derive from the record — the release manifest, the flavor's configuration, the distribution registry, and the release-delivery URL layout.

#### Scenario: A release renders without hand edits

- **WHEN** a release channel publishes a new version and a distribution channel's package manifest is rendered for it
- **THEN** the flavored package manifest names that version, its artifacts' digests, and their download URLs, with no file under `distribution/` edited

#### Scenario: A flavored package manifest is refused in the tree

- **WHEN** a file under `distribution/`, outside the release index, is committed containing a concrete published version string, digest, or versioned download URL
- **THEN** the repository's checks reject it

#### Scenario: Identity has one source

- **WHEN** a flavor's product name or bundle identifier changes in the flavor's configuration
- **THEN** the next render of every distribution channel carrying that release channel reflects the change with no edit under `distribution/`

#### Scenario: Release channels share one structure

- **WHEN** a distribution channel's package manifest is rendered for two release channels
- **THEN** the flavored package manifests differ only in values drawn from the record and the registry, never in which stanzas, sections, or files are present, and the package manifest contains no conditional

#### Scenario: A package manifest stays valid in its own language

- **WHEN** a package manifest is run through the repository's formatter for its file type
- **THEN** the formatter parses and accepts it as it would the flavored package manifest

### Requirement: Rendering is a repository task with an inspectable output

The repository SHALL render any distribution channel's package manifests for any release channel the registry lists on demand — file names and contents alike — into a location outside the committed tree, from the release manifest the download surface serves. Rendering SHALL fail, naming the missing value, when the record lacks something the template needs, rather than emitting a manifest with a blank.

#### Scenario: A render is reproducible

- **WHEN** the same distribution channel and release channel are rendered twice against the same release manifest
- **THEN** the outputs are byte-identical, under the same file names

#### Scenario: A rendered file is named as its package manager requires

- **WHEN** a distribution channel's package manager derives meaning from a manifest's file name (a cask's token, a winget package identifier)
- **THEN** the flavored package manifest carries the name that package manager expects for the registry's identity, and the channel's validator accepts it without renaming

#### Scenario: A missing digest fails the render

- **WHEN** a package manifest needs an artifact digest the release manifest does not carry
- **THEN** rendering fails naming the artifact, and no partial output is written

### Requirement: A package-manager-owned install launches without self-updating

Where a distribution channel's package manager owns the installed files, the package SHALL launch the app with self-updating disabled — through the launcher it installs and through the executable on `PATH` — so that the app notifies about a newer version and the package manager delivers it.

#### Scenario: The AUR package launches notify-only

- **WHEN** the AUR package is installed and the app is started from its desktop entry or from the shell
- **THEN** the app runs with self-updating disabled, and an available update produces a notification rather than an install

#### Scenario: Self-updating channels launch unchanged

- **WHEN** the Homebrew, winget, or Chocolatey package starts the app
- **THEN** no such flag is passed, and the app's own update setting applies

### Requirement: A flavored package manifest is validated with its channel's own tooling

Every package channel SHALL have a validation recipe that checks a flavored package manifest with the channel's own tooling where that tooling runs on a contributor's machine or in CI, and that recipe SHALL run against the render, never the package manifest. A channel whose tooling runs only on its target platform SHALL say so in its recipe rather than silently pass.

#### Scenario: A malformed package manifest fails validation

- **WHEN** a package manifest is edited so that its render violates the channel's schema or style
- **THEN** the channel's validation recipe fails naming the violation

#### Scenario: Validation is not host-dependent in disguise

- **WHEN** a channel's validator cannot run on the current host
- **THEN** the recipe reports that it was skipped and why, rather than succeeding without checking
