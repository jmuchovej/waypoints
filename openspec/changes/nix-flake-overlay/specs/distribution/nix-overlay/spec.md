## Purpose

Defines how Waypoints desktop builds are delivered through Nix: the flake overlay's package set, the flavor-to-package mapping, platform wrapper layout on macOS and Linux, and the update behavior of store-installed builds.

## ADDED Requirements

### Requirement: Flake overlay exposes desktop packages

The repository SHALL be consumable as a Nix flake providing `overlays.default`. Applying the overlay to a consumer's nixpkgs SHALL add the package attributes `waypoints`, `waypoints-preview`, and `waypoints-nightly`. The overlay MUST be self-contained: it SHALL resolve every dependency through its own `final`/`prev` arguments and MUST NOT require the consumer to add any further flake inputs or overlays.

#### Scenario: Consumer installs via their own nixpkgs

- **WHEN** a consumer applies `overlays.default` to their nixpkgs instance and evaluates `pkgs.waypoints`
- **THEN** the stable-flavor desktop app builds using the consumer's nixpkgs, with no Waypoints-specific inputs required beyond the Waypoints flake itself

#### Scenario: Direct flake build

- **WHEN** a user runs `nix build <flake-ref>#waypoints` (or `#waypoints-preview`, `#waypoints-nightly`) on a supported system
- **THEN** the corresponding package builds and produces an installable output

### Requirement: Flavor channels map to distinct co-installable packages

`waypoints` SHALL build the stable flavor, `waypoints-preview` the preview flavor, and `waypoints-nightly` the nightly flavor. Each package SHALL carry its flavor's product name, bundle identifier, icons, and update endpoint. Because flavors have distinct product names and identifiers, any combination of the three packages SHALL be installable side by side without file or identity collisions. The development flavor SHALL NOT be exposed as a package.

A package's version SHALL come from the app manifest (`src-tauri/Cargo.toml`), which tracks the version of the branch it is built from. Flavor configs define channel identity only and SHALL NOT declare a `version`, because a Tauri config `version` overrides the manifest and would pin every branch's bundle to a single value.

#### Scenario: Version follows the manifest, not the flavor

- **WHEN** a package of any flavor is built from a branch whose app manifest declares version `X`
- **THEN** the resulting bundle reports version `X`, and no flavor config overrides it

#### Scenario: Nightly and stable coexist

- **WHEN** a user installs both `waypoints` and `waypoints-nightly` in the same profile
- **THEN** both apps install and launch independently, each under its own product name and bundle identifier

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
