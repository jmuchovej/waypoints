## Purpose

Defines how Waypoints desktop builds are delivered through Nix: the flake overlay's package set, the flavor-to-package mapping, platform wrapper layout on macOS and Linux, and the update behavior of store-installed builds.

## Requirements

### Requirement: Flake overlay exposes desktop packages

The repository SHALL be consumable as a Nix flake providing `overlays.default`. Applying the overlay to a consumer's nixpkgs SHALL add the package attributes `waypoints`, `waypoints-preview`, and `waypoints-nightly` for the desktop app, and `outpost` for the sync server's command-line binary. The overlay MUST be self-contained: it SHALL resolve every dependency through its own `final`/`prev` arguments and MUST NOT require the consumer to add any further flake inputs or overlays.

The `outpost` package SHALL build from the tree the flake is evaluated from and SHALL carry the version of the `outpost` crate manifest. Its runtime closure SHALL NOT include the desktop app's GUI toolkit, a JavaScript runtime, or frontend assets: it is a headless server binary. It SHALL expose `outpost` as its main program.

#### Scenario: Consumer installs via their own nixpkgs

- **WHEN** a consumer applies `overlays.default` to their nixpkgs instance and evaluates `pkgs.waypoints`
- **THEN** the stable-flavor desktop app builds using the consumer's nixpkgs, with no Waypoints-specific inputs required beyond the Waypoints flake itself

#### Scenario: Direct flake build

- **WHEN** a user runs `nix build <flake-ref>#waypoints` (or `#waypoints-preview`, `#waypoints-nightly`) on a supported system
- **THEN** the corresponding package builds and produces an installable output

#### Scenario: Server package builds headless

- **WHEN** a user runs `nix build <flake-ref>#outpost` on a supported system, or evaluates `pkgs.outpost` with the overlay applied
- **THEN** the output contains `bin/outpost`, which runs `outpost station`
- **AND** the output's runtime closure contains no WebKitGTK, Node.js, pnpm, or frontend source tree

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

### Requirement: Platform wrapper layout

On macOS (aarch64-darwin), a package's output SHALL contain `Applications/<ProductName>.app` — a structurally valid application bundle (executable under `Contents/MacOS/`, `Contents/Info.plist` declaring the flavor's bundle identifier and URL schemes, flavor icon under `Contents/Resources/`) — so that standard app-linking mechanisms (home-manager, nix-darwin) can place it where LaunchServices registers it. On Linux (x86_64-linux, aarch64-linux), the output SHALL contain the executable under `bin/`, a `.desktop` entry, and hicolor icons.

#### Scenario: macOS install is LaunchServices-linkable

- **WHEN** the darwin package output is linked into `~/Applications` by an app-linking module
- **THEN** the app launches from Finder/Spotlight and its URL scheme registrations are picked up by LaunchServices

#### Scenario: Linux desktop integration

- **WHEN** the Linux package is installed into a profile
- **THEN** the app is launchable from `$PATH` and appears in desktop launchers via its `.desktop` entry and icons

### Requirement: Frontend is embedded, never shipped standalone

Each package SHALL embed the flavor-matched frontend assets into the application binary at build time. The package output SHALL NOT contain a standalone frontend artifact, and the runtime closure SHALL NOT include a JavaScript runtime or package manager.

#### Scenario: App runs self-contained

- **WHEN** the installed app is launched with no network access
- **THEN** the UI renders from embedded assets

#### Scenario: Closure hygiene

- **WHEN** the package's runtime closure is inspected
- **THEN** it contains no Node.js, pnpm, or frontend source tree

### Requirement: Store-installed builds notify about updates but never self-update

A Nix-installed build SHALL NOT download, stage, or apply an update to itself under any circumstance. It SHALL still query its flavor's update endpoint and, when a newer version than the running one is available, surface a notification to the user. The notification SHALL NOT offer an in-place install action.

#### Scenario: Newer version available

- **WHEN** a store-installed build checks its flavor's update endpoint and a newer version exists
- **THEN** the user is notified of the new version and the installation on disk is unchanged

#### Scenario: No self-modification attempted

- **WHEN** a store-installed build runs through an update-check cycle
- **THEN** no write is attempted against the application's own installation path

#### Scenario: Endpoint unreachable

- **WHEN** the update endpoint cannot be reached
- **THEN** the app continues operating normally and no error interrupts the user

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
