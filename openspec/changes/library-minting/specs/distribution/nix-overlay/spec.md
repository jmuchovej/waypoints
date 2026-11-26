## MODIFIED Requirements

### Requirement: Flake overlay exposes desktop packages

The repository SHALL be consumable as a Nix flake providing `overlays.default`. Applying the overlay to a consumer's nixpkgs SHALL add the package attributes `waypoints`, `waypoints-preview`, and `waypoints-nightly` for the desktop app, and `outpost` for the sync server's command-line binary. The overlay MUST be self-contained: it SHALL resolve every dependency through its own `final`/`prev` arguments and MUST NOT require the consumer to add any further flake inputs or overlays.

The `outpost` package SHALL build the `outpost` crate the evaluated tree resolves: on `main`, the in-tree `crates/outpost`; on a series ref whose workspace pins the `outpost` group to a tag, the crate at that tag, so that a series' Station is a build of a released `outpost` and never of the series' inert in-tree copy. It SHALL carry the version of the `outpost` crate manifest it builds. Its runtime closure SHALL NOT include the desktop app's GUI toolkit, a JavaScript runtime, or frontend assets: it is a headless server binary. It SHALL expose `outpost` as its main program.

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

#### Scenario: A series builds the Station from its pin

- **WHEN** a consumer pins a series ref whose lockfile pins `outpost-v0.2.0` and builds `outpost`
- **THEN** the package is built from the `outpost` crate at `outpost-v0.2.0`, carries that manifest's version, and ignores the series' in-tree `crates/outpost`
