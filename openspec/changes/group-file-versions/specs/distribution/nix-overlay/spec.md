## MODIFIED Requirements

### Requirement: Flavor channels map to distinct co-installable packages

The overlay SHALL expose one attribute set, `waypoints`, holding three packages: `waypoints.stable` builds the stable flavor, `waypoints.preview` the preview flavor, and `waypoints.nightly` the nightly flavor. The flake SHALL expose the same three as `stable`, `preview`, and `nightly`, plus `default`. Each package SHALL carry its flavor's product name, bundle identifier, icons, and update endpoint. Because flavors have distinct product names and identifiers, any combination of the three packages SHALL be installable side by side without file or identity collisions. The development flavor SHALL NOT be exposed as a package.

Where a version comes from is governed by `distribution/release-planning`, not by this capability: it is the version the group file (`tooling/release-groups.toml`) declares for the app at the commit being built, which that commit's app manifest projects, and a flavor config SHALL NOT declare a `version`. What this capability adds is _which_ commit that is, and that a publish-time suffix carried by a recorded tag, which appears in no manifest, SHALL be preserved rather than recomputed.

#### Scenario: Version follows the manifest, not the flavor

- **WHEN** a package is built from a commit whose group file declares app version `X`, projected into its app manifest
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
