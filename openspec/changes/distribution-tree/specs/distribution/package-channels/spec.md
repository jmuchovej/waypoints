# Spec Delta

## Purpose

Governs the manifests through which third-party package managers install Waypoints — where they live in the tree, how each derives from the published release record rather than restating it, how the repository declares which distribution channels each release channel reaches and under what identity, and how a rendered manifest is checked before it leaves the tree. A _release channel_ is stable, preview, or nightly; a _distribution channel_ is a package manager or deployment target.

## ADDED Requirements

### Requirement: Every distribution channel lives under one tree

The repository SHALL hold every distribution channel's files under a single top-level `distribution/` directory, one subdirectory per channel, with the channel's validation recipes reachable from the repository's task runner under one module. The Nix overlay — its recipes, channel reading, and release index — SHALL be one such channel.

#### Scenario: A channel is found by name

- **WHEN** a contributor looks for the files behind a package manager (Homebrew, the AUR, winget, Chocolatey, Nix) or the Station's reference deployment (Kubernetes)
- **THEN** each is a directory directly under `distribution/`, named for the channel

#### Scenario: A channel's inner layout is its own

- **WHEN** a channel has more than one form of file (a kustomize base and its overlays; build recipes and a release index)
- **THEN** those forms nest inside the channel's directory in its tooling's idiom, the channel's recipes sit at the channel's root, and no form becomes a top-level channel of its own

#### Scenario: Validation is one module away

- **WHEN** a contributor lists the task runner's recipes for distribution
- **THEN** every channel that has validation recipes appears as a submodule of one distribution module, and no channel's recipes live under any other module

### Requirement: A distribution registry names each release channel's distribution channels and identities

The repository SHALL hold one registry at the root of `distribution/` with a table per release channel, each entry naming a distribution channel and the package identity that release channel takes there, following the package manager's convention for variants of one application. A release channel absent from a distribution channel's entries SHALL NOT be rendered for it. Identities SHALL keep release channels co-installable wherever the package manager installs side by side.

#### Scenario: The registry decides eligibility

- **WHEN** a render is requested for a release channel the registry does not list under that distribution channel
- **THEN** rendering fails naming the release channels the registry does list for it

#### Scenario: Identities follow the channel's convention

- **WHEN** a distribution channel's package manager has a naming convention for variants of one application
- **THEN** the registry's identities follow it (a versioned-cask suffix, a `-bin` suffix, a dotted identifier segment), and the templates read them from the registry rather than spelling them out

#### Scenario: Release channels are distinct packages

- **WHEN** a distribution channel lists more than one release channel
- **THEN** each renders to a distinct package identity, and installing two does not conflict on executable name, application name, or install location

#### Scenario: The registry is the single list

- **WHEN** a release channel is added to or removed from a distribution channel
- **THEN** only the registry changes, and renders for a removed release channel are refused from then on

### Requirement: Channel manifests are templates, derived from the release record

A package channel's directory SHALL hold templates, never a rendered manifest: outside the release index, no committed file under `distribution/` names a published version, digest, or versioned download URL. Every release-specific value in a render SHALL derive from the published record — the channel manifest, the flavor's configuration, the distribution registry, and the release-delivery URL layout.

#### Scenario: A release renders without hand edits

- **WHEN** a release channel publishes a new version and a distribution channel's manifest is rendered for it
- **THEN** the rendered manifest names that version, its artifacts' digests, and their download URLs, with no file under `distribution/` edited

#### Scenario: A stamped manifest is refused

- **WHEN** a file under `distribution/`, outside the release index, is committed containing a concrete published version string, digest, or versioned download URL
- **THEN** the repository's checks reject it as a rendered manifest in the template tree

#### Scenario: Identity has one source

- **WHEN** a flavor's product name or bundle identifier changes in the flavor's configuration
- **THEN** the next render of every distribution channel carrying that release channel reflects the change with no edit under `distribution/`

#### Scenario: Release channels share one structure

- **WHEN** a distribution channel's templates are rendered for two release channels
- **THEN** the renders differ only in values drawn from the record and the registry, never in which stanzas, sections, or files are present, and the templates themselves contain no conditional

#### Scenario: A template stays valid in its own language

- **WHEN** a distribution channel's template is run through the formatter and linter for its file type
- **THEN** they parse and accept it as they would the rendered manifest

### Requirement: Rendering is a repository task with an inspectable output

The repository SHALL render any distribution channel's templates for any release channel the registry lists on demand — file names and contents alike — into a location outside the committed tree, from the channel manifest the download surface serves. Rendering SHALL fail, naming the missing value, when the record lacks something the template needs, rather than emitting a manifest with a blank.

#### Scenario: A render is reproducible

- **WHEN** the same distribution channel and release channel are rendered twice against the same published manifest
- **THEN** the outputs are byte-identical, under the same file names

#### Scenario: A rendered file is named as its package manager requires

- **WHEN** a distribution channel's package manager derives meaning from a manifest's file name (a cask's token, a winget package identifier)
- **THEN** the rendered file carries the name that package manager expects for the registry's identity, and the channel's validator accepts it without renaming

#### Scenario: A missing digest fails the render

- **WHEN** a template needs an artifact digest the channel manifest does not carry
- **THEN** rendering fails naming the artifact, and no partial manifest is written

### Requirement: A rendered manifest is validated with its channel's own tooling

Every package channel SHALL have a validation recipe that checks a rendered manifest with the channel's own tooling where that tooling runs on a contributor's machine or in CI, and that recipe SHALL run against the rendered output, never the template. A channel whose tooling runs only on its target platform SHALL say so in its recipe rather than silently pass.

#### Scenario: A malformed template fails validation

- **WHEN** a template is edited so that its rendered manifest violates the channel's schema or style
- **THEN** the channel's validation recipe fails naming the violation

#### Scenario: Validation is not host-dependent in disguise

- **WHEN** a channel's validator cannot run on the current host
- **THEN** the recipe reports that it was skipped and why, rather than succeeding without checking

### Requirement: The Station's reference deployment is a channel

The Station's Kubernetes manifests and the kind overlay that exercises them SHALL live as one channel under `distribution/`, the overlay nested beside the base and referring to it by relative path, with the deployment recipes (lint, image, cluster up, image load, apply, cluster down) at the channel's root.

#### Scenario: The overlay renders from its new home

- **WHEN** the kind overlay is rendered with kustomize from its directory inside the channel
- **THEN** it resolves the base manifests from its sibling directory and renders without error

#### Scenario: Recipes keep their behavior

- **WHEN** the channel's recipes are run in order on a host with the deploy tools
- **THEN** a cluster is created, the image loaded, the manifests applied, and the cluster deleted, as before the reorganization
