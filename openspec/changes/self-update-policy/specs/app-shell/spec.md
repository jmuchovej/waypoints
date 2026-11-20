# Spec Delta

## Purpose

Defines the desktop app's runtime behavior as the Tauri shell owns it, starting with self-updating: the user's setting, the launch flag through which an installing package caps it, and what the app tells a user whose install is managed.

## ADDED Requirements

### Requirement: Self-updating is a user setting with three values

The app SHALL keep a `self_update` setting with the values `off`, `notify`, and `on`, defaulting to `on`, stored with the app's other settings and editable on the settings page. `off` SHALL perform no update check; `notify` SHALL check and announce a newer version without downloading or installing it; `on` SHALL check, download, and apply.

#### Scenario: Default behavior

- **WHEN** a fresh install checks for updates with no setting stored
- **THEN** it behaves as `on`

#### Scenario: Notify announces and leaves the install alone

- **WHEN** the effective policy is `notify` and a newer version exists
- **THEN** the user is told the version is available, and no write is attempted against the application's own installation path

#### Scenario: Off performs no check

- **WHEN** the effective policy is `off`
- **THEN** the update endpoint is not contacted

### Requirement: An installing package caps the policy at launch without touching the setting

The app SHALL accept the launch flag `--no-self-update`, and `--self-update` as its explicit inverse, anywhere in its arguments and alongside any deep-link argument. With `--no-self-update`, the effective policy SHALL be the lesser of the stored setting and `notify`; the stored setting SHALL NOT be written. Arguments forwarded from a second instance SHALL NOT change the running instance's policy.

#### Scenario: A capped install with the default setting

- **WHEN** the app is launched with `--no-self-update` and the stored setting is `on`
- **THEN** it behaves as `notify`, and the stored setting still reads `on`

#### Scenario: Off survives the cap

- **WHEN** the app is launched with `--no-self-update` and the stored setting is `off`
- **THEN** it behaves as `off`

#### Scenario: The flag coexists with a deep link

- **WHEN** the app is launched as `<executable> --no-self-update waypoints://pair/…`
- **THEN** the policy is capped and the deep link is handled as it would be without the flag

#### Scenario: A forwarded argument list does not re-decide the policy

- **WHEN** a second instance starts without the flag and forwards its arguments to a running instance that was launched with it
- **THEN** the running instance stays capped

### Requirement: A capped install says so

When the policy is capped, the settings page SHALL show the `on` option disabled, with a notice that self-updating is disabled because this copy is managed by a package manager, which delivers updates itself, and a link to the page that lists the managed channels. The notice SHALL NOT name a distribution or package manager.

#### Scenario: Settings page under a cap

- **WHEN** a user opens the self-update setting on an install launched with `--no-self-update`
- **THEN** `on` is disabled, the notice is shown with its link, and `off` and `notify` remain selectable

#### Scenario: Settings page without a cap

- **WHEN** a user opens the self-update setting on an install launched without the flag
- **THEN** all three values are selectable and no notice is shown

### Requirement: One binary serves every channel

The app SHALL NOT vary its update behavior by build configuration: a published artifact and a store-built binary of the same release SHALL differ in update behavior only by the launch flag.

#### Scenario: The same artifact, two launches

- **WHEN** the same binary is launched once with `--no-self-update` and once without
- **THEN** the first behaves as `notify` at most and the second as its stored setting
