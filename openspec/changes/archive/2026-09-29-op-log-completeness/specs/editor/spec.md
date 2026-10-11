## ADDED Requirements

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

## Decisions

### 2026-07-30 — Per-field operations for landmark titles and readings

Landmark titles and readings get their own operation kinds and their own
contention entities, rather than being carried by the whole-value Mark-data
operation.

_Rationale:_ a landmark holds four independent fields. One whole-value operation
conflates them, so inverting a title edit also reverts any reading edit captured
in the same snapshot. Content marks do not have this problem — a block's text is
genuinely one value — so the whole-value operation remains correct there.

_Rejected:_ keeping every landmark edit on the whole-value operation. It cannot
express independent inversion.

_Rejected:_ splitting the remaining landmark fields. Icon and chart are
low-traffic; splitting them now would be speculative. They remain coarse.

### 2026-07-30 — Validation precedes operation construction; application is mechanical

Reading validation runs once, before an operation exists. Applying a recorded
operation performs no validation.

_Rationale:_ application is also the undo and redo path. A validating apply could
fail partway through reversing a group, leaving persisted state partly reverted
with the group already consumed.

_Rejected:_ validating on apply with transactional rollback of the undo group.
Correct, but it requires transactional undo — a substantially larger contract
than this capability needs.

### 2026-07-30 — Null reading encodes absence

A null reading means "this feature is unset". Setting null removes the reading;
a previously unset feature records null as its prior value.

_Rationale:_ inversion becomes a straight swap of previous and new with no
optionality layer, and it matches the semantics already exposed to callers.

_Rejected:_ an optional previous-reading field. It adds a second way to say
"absent" that must be kept consistent with the null reading.

### 2026-07-30 — A single undo authority

Undo and redo are answered only from the history held alongside persisted state.
Client-side editing surfaces do not answer them from local stacks.

_Rationale:_ once text edits are recorded centrally, an edit present in two
stacks is reversed twice. Independently, per-Mark local stacks drain before any
centrally recorded operation regardless of when the edits occurred, so undo
does not follow the user's actual edit order.

_Trade-off accepted:_ text undo granularity becomes the coalescing window rather
than the client surface's internal grouping. Correct ordering is worth more than
granularity that reverses the wrong edit.

_Rejected:_ reconciling the two stacks by having the client report its local
undo steps as operations. That is a second op-production path — the same class
of divergence this capability exists to remove.

### 2026-07-30 — Landmark creation records into the new landmark's own history

A created landmark has no containing landmark, so its create operation is
recorded under the new landmark itself.

_Rationale:_ the "every mutation is op-shaped" invariant holds without
exceptions. Every mutation lands in some history.

_Consequence accepted:_ undoing immediately after creating a landmark, while
viewing it, soft-deletes it. This is bounded — the landmark is new and empty —
and redo restores it with its original identity.

_Rejected:_ applying the operation without recording it. That is precisely the
"persists but records nothing" hole this capability closes.

_Rejected:_ a territory-scoped history for landmark-level operations. New
machinery for a single operation kind.

### 2026-08-07 — Operations are locally applied but wire-typed by design

This capability specifies how operations are recorded, inverted, and replayed
against local state. It does not specify transport, and it adds none. It also
does not forbid the operation vocabulary from being a wire type — because it
already is one, deliberately.

_Rationale:_ an earlier draft carried a requirement that operations be
documented as local-undo-only, forbidding any description of them travelling
between devices or being wrapped for transport. That requirement was withdrawn.
It rested on the belief that `Maneuver`'s doc comments described a retired
protocol, which was wrong: the envelope type they name is an
`outpost-protocol` type, and the operation enum was designed as a wire type from
the start — `#[serde(skip)]` on undo-only pre-state exists precisely so those
fields are never transmitted. The in-flight sync change builds directly on that
design and cites those comments as groundwork.

_Why withdrawing it matters:_ as written, the requirement would have made the
next change violate this capability's spec on arrival, forcing its deletion one
change later. A spec that forbids the system's intended direction is worse than
silence on the subject.

_What survives:_ the invariant that actually matters here is already stated in
_Every editor mutation is op-shaped_ — pre-state carried for undo is local-only
and never part of any serialized form of the operation. That holds regardless of
whether operations are transmitted.

_Scope, unchanged:_ this change serializes nothing and publishes nothing. Sync
is a separate change. That is a boundary on the work, not a claim about what the
operation vocabulary is.

### 2026-08-06 — Projection's containment authority is Terrain, not Landmark data

Projection reads Terrain child edges and nothing else to determine what a
Landmark contains and in what order.

_Rationale:_ this is the projection-path consequence of a data-model invariant
that the `data-model` capability owns — a Landmark's content is expressed solely
by Terrain child edges, and the child-reference field Landmark data used to
carry is removed. The full decision, including why the field was deleted rather
than annotated and why its reference type survives, lives in the `data-model`
spec's Decisions log. It is restated here only as the requirement above, because
projection is this capability's to specify.

_Why it matters to this capability:_ the premise here is that every state change
is expressible as an invertible operation. A field nothing operates on cannot
participate in that. Containment changes are edge mutations, which the operation
vocabulary already covers.
