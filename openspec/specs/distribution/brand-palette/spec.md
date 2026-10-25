# distribution/brand-palette Specification

## Purpose

Names every brand color exactly once, in a palette every brand surface imports, so that an app icon, a store asset, and any future brand surface cannot disagree about what a flavor looks like.

## Requirements

### Requirement: A brand color is named once

Each flavor's colors SHALL be declared in exactly one committed palette. Every renderer that draws a brand surface SHALL take its colors from that palette. A renderer SHALL NOT carry its own declaration of a color the palette already names, whether as a literal value or as a binding that restates one.

A renderer MAY derive a view over the palette — an ordering, a subset, a role name — provided the color values in that view come from the palette.

#### Scenario: Two surfaces draw the same flavor

- **WHEN** an app icon and a store asset are drawn for the same flavor
- **THEN** each takes the flavor's colors from the one palette, and neither contains a color value of its own

#### Scenario: A flavor's color changes

- **WHEN** a color is changed in the palette and every renderer runs
- **THEN** every surface that uses that color reflects the change, with no second edit anywhere

#### Scenario: A renderer needs colors by role

- **WHEN** a renderer addresses colors by a role its own composition defines, rather than by the names the palette uses
- **THEN** the role resolves to a value taken from the palette, and the role's definition holds no color value

### Requirement: Every flavor's palette has the same shape

Every flavor's palette SHALL declare the same set of entries. A renderer SHALL be able to draw any flavor without knowing which flavor it is drawing. An entry a renderer needs that a flavor's palette lacks SHALL be an error, and SHALL NOT be satisfied by a default, by another flavor's value, or by the artwork's own authored color.

#### Scenario: A flavor is drawn

- **WHEN** a renderer draws any flavor
- **THEN** it finds every entry it needs under the same names as for every other flavor

#### Scenario: A palette is missing an entry

- **WHEN** a flavor's palette omits an entry the other flavors declare and a surface is rendered for it
- **THEN** rendering fails and names the missing entry, and no surface is written

#### Scenario: A flavor is added

- **WHEN** a new flavor's palette is added
- **THEN** every renderer draws it without change, provided it declares the same entries as the others

### Requirement: A palette color that reaches nothing fails loudly

Where a palette color is applied to a named element of the artwork, the named element SHALL exist. A color that targets no element SHALL be an error at render time, naming the color's entry and the element it was meant for. A surface SHALL NOT be written with a color silently unapplied.

#### Scenario: Artwork loses an element

- **WHEN** an element the palette colors is renamed or removed from the artwork
- **THEN** rendering fails and names the element, rather than emitting the surface in the artwork's authored color

#### Scenario: A palette entry is misnamed

- **WHEN** a palette entry targets an element name the artwork does not contain
- **THEN** rendering fails and names both the entry and the missing element
