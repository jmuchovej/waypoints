## MODIFIED Requirements

### Requirement: Flavor channels map to distinct co-installable packages

The overlay SHALL expose one attribute set, `waypoints`, holding three packages: `waypoints.stable` builds the stable flavor, `waypoints.preview` the preview flavor, and `waypoints.nightly` the nightly flavor. The flake SHALL expose the same three as `stable`, `preview`, and `nightly`, plus `default`. Each package SHALL carry its flavor's product name, bundle identifier, icons, and update endpoint. Because flavors have distinct product names and identifiers, any combination of the three packages SHALL be installable side by side without file or identity collisions. The development flavor SHALL NOT be exposed as a package.

Where a version comes from is governed by `distribution/release-planning`, not by this capability: it is read from the app manifest of the commit being built, and a flavor config SHALL NOT declare a `version`. What this capability adds is _which_ commit that is, and that a publish-time suffix carried by a recorded tag, which appears in no manifest, SHALL be preserved rather than recomputed.

#### Scenario: Version follows the manifest, not the flavor

- **WHEN** a package is built from a commit whose app manifest declares version `X`
- **THEN** the resulting bundle reports version `X` — extended by any publish-time suffix the recorded tag carries — and no flavor config overrides it

#### Scenario: Nightly date stamp survives the passage of time

- **WHEN** `waypoints.nightly` is built from `main` on a date later than the one on which its recorded nightly was published
- **THEN** the bundle reports the published nightly's version, counter and date stamp included, rather than recomputing a suffix from the build date

#### Scenario: Nightly and stable coexist

- **WHEN** a user installs both `waypoints.stable` and `waypoints.nightly` in the same profile
- **THEN** both apps install and launch independently, each under its own product name and bundle identifier

#### Scenario: Two spellings, one package

- **WHEN** a consumer evaluates `pkgs.waypoints.preview` through the overlay and `github:jmuchovej/waypoints#preview` through the flake at the same ref
- **THEN** both are the same derivation

#### Scenario: Development flavor is absent

- **WHEN** a consumer inspects the overlay's package set
- **THEN** no package exposing the development flavor exists

## ADDED Requirements

### Requirement: On `main`, each package builds its channel's published source

When the pinned ref's manifest version names the nightly channel, each of the three packages SHALL build the tree the commit its channel's release-index entry names, fetched and verified against the hash that entry records, and evaluated with that tree's own packaging recipe. A package SHALL NOT build the working tree of the ref the consumer pinned.

The pinned ref therefore selects the _index_, not the code: advancing the pinned ref changes which published releases the three packages describe, and never reinterprets the pinned commit as a release of any channel.

#### Scenario: Ref selects the index, not the code

- **WHEN** a consumer pins a ref of `main` and builds `stable`
- **THEN** the build fetches the commit the stable index entry names at that ref, not the pinned commit

#### Scenario: Channel branding never misattributed

- **WHEN** a consumer pins a ref of `main`, whose tree holds nightly-line code, and builds `stable`
- **THEN** the resulting app is the published stable release — not a stable-branded build of `main`'s code — and it does not collide with a stable install from any other source

#### Scenario: Advancing the ref advances the releases

- **WHEN** a consumer advances their pinned ref from one that records release `A` for a channel to one that records release `B`
- **THEN** that channel's package changes from `A` to `B`, and channels whose entries did not change rebuild to identical outputs

#### Scenario: The recorded tree's recipe is used

- **WHEN** the recipe on the pinned `main` commit differs from the recipe at the recorded tag
- **THEN** the package is built with the recorded tag's recipe, and the pinned commit's recipe plays no part

### Requirement: A series ref builds itself

When the pinned ref's manifest version names the preview or stable channel, the package named by that channel SHALL build the pinned tree itself, at the version its app manifest declares. The two other packages SHALL fail to evaluate with a message naming the ref's channel and series and saying where to pin for the missing channel. `default` SHALL be the package the version names.

Pinning a series branch therefore tracks that series' current release as it moves; pinning a tag selects exactly that release.

#### Scenario: A preview series builds its preview

- **WHEN** a consumer pins `v0.2.x` while its manifest declares `0.2.0-pre.N` and builds `preview`
- **THEN** the pinned tree is built as the preview flavor at its manifest version, and `stable` and `nightly` fail to evaluate, the former saying `0.2` has no stable release yet

#### Scenario: A promoted series builds its stable

- **WHEN** a consumer pins `v0.2.x` after its manifest declares a bare `0.2.N` and builds `stable`
- **THEN** the pinned tree is built as the stable flavor at its manifest version, and `preview` fails to evaluate, saying `0.2` ships no further previews and naming `main` or the next series

#### Scenario: A tag is exactly one release

- **WHEN** a consumer pins the tag `v0.2.3` and builds `stable`
- **THEN** the result is the `0.2.3` release, built from the tagged tree with that tree's recipe, lockfiles, and pins

#### Scenario: Nightlies are never built from a series

- **WHEN** a consumer pins any series ref and builds `nightly`
- **THEN** evaluation fails, saying nightlies publish from `main`

#### Scenario: In-tree builds are not channels

- **WHEN** a developer builds a working copy under the development flavor without going through a channel package
- **THEN** the result is not presented as any channel's published release
