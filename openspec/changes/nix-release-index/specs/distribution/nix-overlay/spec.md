## MODIFIED Requirements

### Requirement: Flavor channels map to distinct co-installable packages

`waypoints` SHALL build the stable flavor, `waypoints-preview` the preview flavor, and `waypoints-nightly` the nightly flavor. Each package SHALL carry its flavor's product name, bundle identifier, icons, and update endpoint. Because flavors have distinct product names and identifiers, any combination of the three packages SHALL be installable side by side without file or identity collisions. The development flavor SHALL NOT be exposed as a package.

A package's version SHALL come from the release tag recorded in its channel's release-index entry, and SHALL be the version that channel actually published. It SHALL NOT be taken from the app manifest of the ref the consumer pinned, and SHALL NOT be derived from the build's own wall-clock date. Flavor configs define channel identity only and SHALL NOT declare a `version`, because a Tauri config `version` overrides the manifest and would pin every branch's bundle to a single value.

#### Scenario: Version follows the channel's published tag

- **WHEN** a package is built and its channel's index entry records tag `vX`
- **THEN** the resulting bundle reports version `X`, and no flavor config overrides it

#### Scenario: Nightly date stamp survives the passage of time

- **WHEN** `waypoints-nightly` is built on a date later than the one on which its recorded nightly was published
- **THEN** the bundle reports the published nightly's version, date stamp included, rather than a version stamped with the build date

#### Scenario: Pinned ref does not set the version

- **WHEN** a consumer pins a ref whose app manifest declares a version different from the recorded tags
- **THEN** each package still reports its own channel's published version, and the pinned manifest's version appears in no package

#### Scenario: Nightly and stable coexist

- **WHEN** a user installs both `waypoints` and `waypoints-nightly` in the same profile
- **THEN** both apps install and launch independently, each under its own product name and bundle identifier

#### Scenario: Development flavor is absent

- **WHEN** a consumer inspects the overlay's package set
- **THEN** no package exposing the development flavor exists

## ADDED Requirements

### Requirement: Each package builds its channel's published source

Each of the three packages SHALL build the source of the commit its channel's release-index entry names, fetched and verified against the hashes that entry records. A package SHALL NOT build the working tree or flake source of the ref the consumer pinned.

The pinned ref therefore selects the _index_, not the code: advancing the pinned ref changes which published releases the three packages describe, and never reinterprets the pinned commit as a release of any channel.

#### Scenario: Ref selects the index, not the code

- **WHEN** a consumer pins a ref of the Waypoints flake and builds `waypoints`
- **THEN** the build fetches the commit the stable index entry names at that ref, not the pinned commit

#### Scenario: Channel branding never misattributed

- **WHEN** a consumer pins a ref of `main`, whose tree holds nightly-line code, and builds `waypoints`
- **THEN** the resulting app is the published stable release — not a stable-branded build of `main`'s code — and it does not collide with a stable install from any other source

#### Scenario: Advancing the ref advances the releases

- **WHEN** a consumer advances their pinned ref from one that records release `A` for a channel to one that records release `B`
- **THEN** that channel's package changes from `A` to `B`, and channels whose entries did not change rebuild to identical outputs

#### Scenario: In-tree builds are not channels

- **WHEN** a developer builds from a local working copy without going through the index
- **THEN** the result is not presented as any channel's published release
