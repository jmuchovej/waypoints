# editor Specification

## Purpose

TBD - created by archiving change op-log-completeness. Update Purpose after archive.

## Requirements

### Requirement: Every editor mutation is op-shaped

Every mutation the editor performs on a Mark or a Terrain edge SHALL produce one
or more operations, and those operations SHALL be recorded in the undo history
before the mutation is reported as complete. No editor entry point may change
persisted state without producing a corresponding operation.

An operation records enough pre-state to reconstruct the state that preceded it.
Pre-state carried for this purpose is local-only and is never part of any
serialized form of the operation.

#### Scenario: A mutation that persists also records

- **WHEN** any editor entry point completes successfully and has changed
  persisted state
- **THEN** the undo history for the affected landmark contains at least one
  operation describing that change

#### Scenario: A mutation that fails records nothing

- **WHEN** an editor entry point rejects its input or cannot find the target
- **THEN** persisted state is unchanged
- **AND** no operation is added to any undo history

#### Scenario: Undo reverses a completed mutation

- **WHEN** a mutation completes and undo is requested for the same landmark
- **THEN** the persisted state matches the state before that mutation
- **AND** redo restores the state after that mutation

### Requirement: Landmark title edits are undoable

Setting a landmark's title SHALL produce a title operation carrying the previous
and new title. Undo SHALL restore the previous title; redo SHALL reapply the new
one. The operation SHALL be recorded in the undo history of the landmark whose
title changed.

Setting a title on a Mark that is not a landmark SHALL be rejected and SHALL
leave persisted state unchanged.

#### Scenario: Undo restores the previous title

- **WHEN** a landmark's title is changed from A to B and undo is requested
- **THEN** the landmark's title is A
- **AND** the landmark's chart, icon, and readings are unchanged

#### Scenario: Successive title edits undo one at a time

- **WHEN** a landmark's title is changed from A to B, then from B to C, and undo
  is requested twice
- **THEN** the title after the first undo is B and after the second is A

#### Scenario: Title on a content mark is rejected

- **WHEN** a title is set on a Mark holding block content
- **THEN** the request is rejected
- **AND** no operation is recorded

### Requirement: Reading edits are undoable and independently targeted

Setting or clearing a reading on a landmark SHALL produce a reading operation
carrying the landmark, the feature, and the previous and new readings. Undo
SHALL restore the previous reading; redo SHALL reapply the new one.

A reading operation SHALL identify the specific feature it changes, so that
operations against different features of the same landmark address distinct
targets and do not describe each other's state.

The absence of a reading SHALL be represented as a null reading. Setting a
reading to null SHALL remove it; a previously unset feature SHALL record null as
its previous value.

#### Scenario: Undo restores the previous reading

- **WHEN** a reading on feature F of a landmark is changed and undo is requested
- **THEN** feature F holds its previous reading
- **AND** every other reading on that landmark is unchanged

#### Scenario: Undo restores a cleared reading

- **WHEN** a reading on feature F is cleared and undo is requested
- **THEN** feature F holds the reading it had before being cleared

#### Scenario: Undo removes a newly set reading

- **WHEN** a reading is set on a feature that previously had none and undo is
  requested
- **THEN** that feature has no reading

#### Scenario: Edits to two features undo independently

- **WHEN** feature F is set, then feature G is set, and undo is requested once
- **THEN** feature G is restored to its previous value
- **AND** feature F retains its newly set value

### Requirement: Reading validation happens before the operation is recorded

A reading SHALL be validated against its feature's schema — including type
shape, feature-specific constraints, and the allowed chart types of reference
readings — before any operation is produced. A reading that fails validation
SHALL be rejected, SHALL leave persisted state unchanged, and SHALL record no
operation.

Applying an already-recorded operation SHALL NOT revalidate. An operation that
was valid when recorded remains applicable, so that undo cannot fail because a
Chart's constraints were narrowed afterwards.

#### Scenario: Invalid reading is rejected

- **WHEN** a reading whose shape does not match its feature's kind is submitted
- **THEN** the request is rejected
- **AND** the landmark's readings are unchanged
- **AND** no operation is recorded

#### Scenario: Disallowed reference target is rejected

- **WHEN** a reference reading names a landmark whose chart is not among the
  feature's allowed types
- **THEN** the request is rejected and no operation is recorded

#### Scenario: Undo succeeds after constraints narrow

- **WHEN** a reading is set, the feature's constraints are later narrowed so
  that the earlier reading would no longer validate, and undo is requested
- **THEN** the undo succeeds and restores the earlier state

### Requirement: Text edits are recorded in the undo history

Updating the rich text of a content Mark SHALL produce a rich-text operation
carrying the previous and new text, recorded in the undo history of the landmark
being edited.

Consecutive text operations against the same Mark within the history's
coalescing window SHALL merge into a single undo group, so that a burst of
typing is reversed as one step. Structural mutations SHALL always begin a new
group and SHALL never coalesce.

#### Scenario: Undo restores the previous text

- **WHEN** a content Mark's text is changed and undo is requested
- **THEN** the Mark holds its previous text

#### Scenario: A typing burst is one undo step

- **WHEN** several text updates are applied to the same Mark within the
  coalescing window and undo is requested once
- **THEN** the Mark holds the text it had before the first update in that burst

#### Scenario: A structural edit interrupts coalescing

- **WHEN** a text update is followed by a structural mutation and then another
  text update, and undo is requested once
- **THEN** only the second text update is reversed

### Requirement: The backend is the sole undo authority

The undo history maintained alongside persisted state SHALL be the only authority
for undo and redo. No client-side editing surface may satisfy an undo or redo
request from a local stack of its own.

Undo and redo SHALL reverse and replay mutations in the order the user performed
them across all Marks within a landmark, not grouped by the Mark that holds
focus.

#### Scenario: Undo follows global edit order

- **WHEN** text is edited in Mark A, then a structural change is made, then text
  is edited in Mark B, and undo is requested three times
- **THEN** the reversals occur in the order: Mark B's text, the structural
  change, Mark A's text

#### Scenario: An edit is reversed exactly once

- **WHEN** a text edit is made and undo is requested once
- **THEN** that edit is reversed
- **AND** a subsequent undo does not reverse it a second time

### Requirement: Landmark creation is an editor mutation

Creating a landmark SHALL go through the editor and SHALL produce a create
operation. The operation SHALL be recorded in the undo history keyed by the newly
created landmark.

Undoing a landmark creation SHALL soft-delete the landmark; redoing it SHALL
restore the same landmark with its original identity.

#### Scenario: Creating a landmark records an operation

- **WHEN** a landmark is created from a chart
- **THEN** the landmark is persisted
- **AND** the undo history for that landmark contains its create operation

#### Scenario: Redo restores the same identity

- **WHEN** a landmark creation is undone and then redone
- **THEN** the landmark exists again with the identity it was originally given

### Requirement: Operations name the state they contend for

Every operation SHALL expose the entity whose state it changes. Operations that
change independent fields of the same Mark SHALL expose distinct entities, so
that two such operations are never treated as describing the same state.

Landmark titles and individual landmark readings each name their own entity.
Whole-Mark operations name the Mark. Edge operations name the edge.

#### Scenario: Two features of one landmark are distinct entities

- **WHEN** two reading operations change different features of the same landmark
- **THEN** the entities they name are different

#### Scenario: A title and a reading on one landmark are distinct entities

- **WHEN** a title operation and a reading operation change the same landmark
- **THEN** the entities they name are different

### Requirement: Projection derives content from Terrain edges alone

Projecting a Landmark into an ordered block list SHALL determine both membership
and order from Terrain child edges, and SHALL NOT consult Landmark data for
either. This capability restates, for the projection path it owns, the
containment invariant owned by the `data-model` capability.

Structural mutations that change what a Landmark contains or how its contents
are ordered SHALL express that change as Terrain edge state.

#### Scenario: Projection reads edges, not landmark data

- **WHEN** a Landmark is projected into its block list
- **THEN** membership and order are determined by that Landmark's Terrain child
  edges
- **AND** no field of the Landmark's data contributes to either

#### Scenario: Reordering is edge state

- **WHEN** two blocks within a Landmark are reordered
- **THEN** the resulting projection reflects the new order
- **AND** the Landmark's chart, title, icon, and readings are unchanged
