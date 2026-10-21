## Purpose

Defines what a flavor's app icons are generated from and how they stay in step with those sources, so the icon a user installs is always the one the committed artwork and palette describe.

## ADDED Requirements

### Requirement: Every icon is generated from committed sources by one command

The repository SHALL provide a single command that generates every app icon for a flavor across every platform the flavor ships to. The command SHALL derive every icon from committed sources: the logo artwork, the per-platform masks, the flavor's palette, and the table of sizes and formats each platform requires.

An icon SHALL NOT be produced by any step a contributor must remember separately, and SHALL NOT depend on anything fetched at generation time.

#### Scenario: A flavor is regenerated

- **WHEN** the command runs for a flavor
- **THEN** every icon that flavor ships, on every platform, is written from the committed sources in that one invocation

#### Scenario: A mask changes

- **WHEN** a platform's mask is edited and the command runs
- **THEN** every icon for that platform reflects the edit, and icons for other platforms are unchanged

#### Scenario: Generation is offline

- **WHEN** the command runs with no network available
- **THEN** it succeeds, because every input and every rendering dependency it needs is available without fetching

### Requirement: The authored artwork is colored, never rewritten

Generating an icon SHALL NOT modify the content of any authored artwork. The colors a flavor's palette assigns SHALL reach the artwork by being layered over it, so that the artwork's own authored colors act only as what an unstyled render shows.

An authored file SHALL render meaningfully on its own, without the generator.

#### Scenario: The generator runs

- **WHEN** icons are generated for any flavor
- **THEN** every authored mask and logo master is byte-identical before and after

#### Scenario: Artwork is opened directly

- **WHEN** an authored mask or logo master is opened in a viewer or vector editor, outside the generator
- **THEN** it renders as a complete image in its authored colors

### Requirement: A platform's sizes and formats are data

The sizes, formats, and platform-specific slot identifiers an icon set comprises SHALL be declared as a committed record, not encoded in the generator. Adding, removing, or resizing a slot SHALL be an edit to that record.

Where a platform requires a manifest describing its icon set, that manifest SHALL be generated from the same record as the images it describes, so a manifest cannot reference an image that was not written.

#### Scenario: A slot is added

- **WHEN** a platform requires an additional size
- **THEN** it is added to the record, and the next run writes it without any change to how generation works

#### Scenario: A manifest and its images

- **WHEN** a platform's icon set is generated
- **THEN** every entry in its manifest names an image that was written in the same run, and every image written for that platform is reachable from the manifest

#### Scenario: Two slots want the same pixels

- **WHEN** a platform's record declares two slots that differ only in how the platform labels them, at the same pixel size
- **THEN** both slots are satisfied, without writing the same image twice

### Requirement: Authored inputs and generated outputs are separately addressable

Sources a person writes SHALL be addressable separately from artifacts a command produces. No location SHALL hold both. A contributor or a command SHALL be able to name every generated icon without naming any source, and every source without naming any generated icon.

Where a platform's toolchain requires generated icons at a location that toolchain owns, those icons MAY live there; such locations SHALL NOT also hold authored sources. Files the generator does not produce SHALL sit outside the set it owns.

#### Scenario: Outputs are named as a set

- **WHEN** every generated icon the repository owns is to be removed or replaced
- **THEN** they can be named as a set, without naming a mask, a palette, a size record, the renderer, or the logo artwork

#### Scenario: A source is added

- **WHEN** a new authored source is added
- **THEN** it lands where sources live, and no generated artifact sits beside it

#### Scenario: A platform toolchain owns a location

- **WHEN** a platform requires its icons inside a project directory that platform's toolchain generates and manages
- **THEN** the icons are written there, and neither the directory's authored files nor its other resources are touched by generation

#### Scenario: A file the generator does not produce

- **WHEN** a file sits near the generated set but is not produced by the generator
- **THEN** generation neither overwrites nor removes it

### Requirement: Generation replaces the generated set whole

Generation SHALL complete before any committed output changes. A run that fails SHALL leave the committed outputs exactly as they were. A run that succeeds SHALL leave the generated set equal to what the sources describe — every output present, and no output the sources no longer describe.

#### Scenario: A run fails partway

- **WHEN** generation fails after some icons have been rendered
- **THEN** no committed output has changed

#### Scenario: A slot is removed

- **WHEN** a slot is removed from the size record and generation runs
- **THEN** the file that slot produced no longer exists, and nothing else is removed

### Requirement: Generated icons are committed and held in lockstep with their sources

Every generated icon the repository owns SHALL be committed. Each SHALL be covered by the repository's generated-file lockstep check: the check regenerates the icons and fails on any difference against the committed tree, including a committed output the sources no longer describe.

Running generation against unchanged sources SHALL produce no difference in the committed tree. A source edited without its icons being regenerated, or an icon edited by hand, SHALL fail the check. The check SHALL run on any change to a source or to a committed icon.

#### Scenario: Sources changed without regeneration

- **WHEN** a palette or mask changes and the commit does not include the regenerated icons
- **THEN** the lockstep check fails and names the stale icons

#### Scenario: An icon is hand-edited

- **WHEN** a committed icon is replaced by hand and the check runs
- **THEN** the check fails, because generation does not reproduce the edit

#### Scenario: An orphaned output is committed

- **WHEN** a slot is removed from the size record but its old output remains committed
- **THEN** the check fails and names the orphaned file

#### Scenario: Regenerated and committed together

- **WHEN** a source changes and the commit includes the regenerated icons
- **THEN** the check passes

#### Scenario: Generation is deterministic

- **WHEN** generation runs twice over unchanged sources
- **THEN** both runs produce identical bytes

### Requirement: A flavor's icons are identified by the flavor, and a build resolves them by that identity

Every icon SHALL be attributable to the flavor it belongs to from its location or name alone. A flavor's build SHALL resolve its icons by that identity, and SHALL fail rather than fall back if an icon it names is absent.

Flavors SHALL NOT share a generated icon. Adding a flavor SHALL require no change to how icons are addressed.

#### Scenario: A build names its icons

- **WHEN** a flavor is built
- **THEN** it resolves the icons belonging to that flavor, and no icon belonging to another

#### Scenario: An icon is missing

- **WHEN** a flavor's build names an icon that has not been generated
- **THEN** the build fails and names it, rather than bundling a different flavor's icon or none

#### Scenario: A flavor is added

- **WHEN** a new flavor is introduced
- **THEN** its icons are addressed by the same scheme as every other flavor's
