## Purpose

Names every brand color exactly once, in a record each renderer reads, so that an app icon, a store asset, and any future brand surface cannot disagree about what a flavor looks like.

## ADDED Requirements

### Requirement: A brand color is named once

Each flavor's colors SHALL be declared in exactly one committed record. Every renderer that draws a brand surface SHALL resolve its colors from that record. A renderer SHALL NOT carry its own declaration of a color the record already names, whether as a literal value or as a binding that restates one.

A renderer MAY derive a view over the record — an ordering, a subset, a role name — provided the color values in that view come from the record.

#### Scenario: Two renderers draw the same flavor

- **WHEN** two renderers draw a brand surface for the same flavor
- **THEN** each reads the flavor's colors from the one record, and neither contains a color value of its own

#### Scenario: A flavor's color changes

- **WHEN** a color is changed in the record and every renderer runs
- **THEN** every surface that uses that color reflects the change, with no second edit anywhere

#### Scenario: A renderer needs colors by role

- **WHEN** a renderer addresses colors by a role its own composition defines, rather than by the names the record uses
- **THEN** the role resolves to a value taken from the record, and the role's definition holds no color value

### Requirement: A token resolves to the same color for every renderer

A color SHALL be written in the record as a token, not as a literal value. A token SHALL name an entry in a shared color scale, optionally qualified by an opacity. Resolving a token SHALL yield the same color for every renderer, to the full precision the output encoding carries.

A token's name SHALL index the shared scale directly, requiring no transformation between the spelling the record uses and the spelling the scale uses.

#### Scenario: The same token in two renderers

- **WHEN** two renderers resolve the same token
- **THEN** both yield the same color, identical in the encoding each writes

#### Scenario: A token carries an opacity

- **WHEN** a token names an entry and an opacity
- **THEN** every renderer yields that entry's color at that opacity, and the opacity is the same value in each

#### Scenario: A token is looked up

- **WHEN** a renderer resolves a token
- **THEN** the token's name is used against the shared scale as written, with no per-renderer rewriting or parsing of the name

### Requirement: An unresolvable token fails loudly

A token that names no entry in the shared color scale SHALL be an error at the point of resolution. A renderer SHALL NOT substitute a default, nearest, or transparent color for a token it cannot resolve, and SHALL NOT emit a surface that omits the element the token was for.

The error SHALL name the token that could not be resolved.

#### Scenario: A misspelled token

- **WHEN** a record names a token that the shared scale does not contain
- **THEN** resolution fails and names that token, and no surface is written

#### Scenario: A scale entry disappears

- **WHEN** the shared color scale stops carrying an entry a record still names
- **THEN** resolution fails rather than falling back, so the stale token is found before a surface ships

### Requirement: Every renderer's view of the shared scale agrees

Where a renderer cannot read the shared color scale from its upstream source and instead carries a copy, that copy SHALL agree with the upstream source for every entry any record names. Disagreement SHALL be detectable without rendering a surface and comparing it by eye.

A copy of the scale SHALL be derived from the upstream source rather than transcribed.

#### Scenario: Upstream retunes a color

- **WHEN** the upstream color scale changes the value of an entry a record names, and a renderer's copy is not updated
- **THEN** the disagreement is reported, rather than surfacing as two renderers drawing different colors

#### Scenario: A copy is checked

- **WHEN** a carried copy of the scale is compared against the upstream source
- **THEN** every entry that any record names is found to hold the same value in both

#### Scenario: A record names a new token

- **WHEN** a record begins naming an entry that was not previously used
- **THEN** that entry is covered by the agreement check from then on, with no separate registration step
