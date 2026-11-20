# Spec Delta

## MODIFIED Requirements

### Requirement: Store-installed builds notify about updates but never self-update

A Nix-installed build SHALL NOT download, stage, or apply an update to itself under any circumstance. The package SHALL ensure this by launching the app with `--no-self-update` through the wrapper it installs, so that the same binary a published artifact ships is capped at notify-only on Nix. It SHALL still query its flavor's update endpoint and, when a newer version than the running one is available, surface a notification to the user. The notification SHALL NOT offer an in-place install action.

#### Scenario: Newer version available

- **WHEN** a store-installed build checks its flavor's update endpoint and a newer version exists
- **THEN** the user is notified of the new version and the installation on disk is unchanged

#### Scenario: No self-modification attempted

- **WHEN** a store-installed build runs through an update-check cycle
- **THEN** no write is attempted against the application's own installation path

#### Scenario: Endpoint unreachable

- **WHEN** the update endpoint cannot be reached
- **THEN** the app continues operating normally and no error interrupts the user

#### Scenario: The cap comes from the wrapper

- **WHEN** the package's wrapper on `PATH` or its desktop entry starts the app
- **THEN** the app receives `--no-self-update`, and a user who selects `on` in settings is shown the managed-install notice rather than an in-place update
