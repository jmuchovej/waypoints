# sync Specification

## Purpose

TBD - created by archiving change sync-mark-graph. Update Purpose after archive.

## Requirements

### Requirement: Editor operations are published to the Territory

Every operation that mutates the Mark/Terrain graph SHALL be published to the Territory as an encrypted Notice carrying `payload_type: "waypoints.page.v1"`. A Notice payload SHALL be a batch of one or more operations. Operations that are applied locally but not yet published SHALL remain durably queued until publication succeeds.

#### Scenario: A local edit becomes a Notice

- **WHEN** a user mutation is applied to the local graph
- **THEN** the resulting operations are durably queued for publication
- **AND** they are published as a `waypoints.page.v1` Notice on the next sync

#### Scenario: Crash between local apply and publication

- **WHEN** the application terminates after an operation is applied locally but before its Notice is published
- **THEN** the operation is still queued on restart
- **AND** it is published on the next sync, so no user edit is lost

#### Scenario: Re-publication is harmless

- **WHEN** an operation is published more than once
- **THEN** every recipient reaches the same state as if it had been published once

### Requirement: Inbound Notices fold into the graph through the same write path as local edits

Notices received from the Territory SHALL be decrypted, decoded into operations, and applied to the Mark/Terrain graph through the same application path used by local operations. There SHALL NOT be a separate remote-only write path.

#### Scenario: A remote operation reaches the graph

- **WHEN** a Notice authored on another device is received and its payload decrypts
- **THEN** its operations are applied to the local Mark/Terrain graph
- **AND** the resulting rows are indistinguishable from rows written by an equivalent local edit

#### Scenario: A Notice that cannot be decrypted

- **WHEN** a received Notice's payload cannot be decrypted with any held read key
- **THEN** the Notice is retained and no partial state is written

### Requirement: Conflicts resolve by per-target last-write-wins

Each operation SHALL address exactly one target. Concurrent operations addressing the same target SHALL be resolved by last-write-wins: the operation with the greatest arbitration key materializes into that target. Operations addressing different targets SHALL NOT interfere.

#### Scenario: Two devices edit the same target concurrently

- **WHEN** two devices concurrently produce operations addressing the same target
- **THEN** both devices materialize the operation with the greater arbitration key
- **AND** both devices agree on which operation that is

#### Scenario: Two devices edit different targets concurrently

- **WHEN** two devices concurrently produce operations addressing different targets
- **THEN** both operations are materialized and neither is lost

### Requirement: The arbitration key is derived from signed content alone

The arbitration key for an operation SHALL be `(hlc, author verifying key bytes, notice id)`, compared lexicographically, where every component is read from the operation's signed Notice header. Deriving the key SHALL NOT depend on local state, arrival time, or any value a device can observe differently from its peers. The order induced by the key SHALL be total.

#### Scenario: Every device derives the same key

- **WHEN** two devices hold the same Notice
- **THEN** each derives an identical arbitration key from it

#### Scenario: Identical clocks do not produce ties

- **WHEN** two Notices carry the same clock value
- **THEN** they are ordered by author key, and by notice id if the author is the same
- **AND** the comparison never reports them equal unless they are the same Notice

### Requirement: The fold is Territory-wide

The fold SHALL consider every Notice in the Territory, not the Notices of a single Track. An operation SHALL be applied regardless of which Track carries it.

#### Scenario: An operation spanning two Landmarks

- **WHEN** an operation moves a Content mark from one Landmark to another, and is carried by exactly one Track
- **THEN** the operation is applied
- **AND** both affected Landmarks reflect the move

#### Scenario: Local arrival counters do not order the fold

- **WHEN** two devices receive the same set of Notices in different sequences
- **THEN** each device's local arrival counter differs
- **AND** neither device's fold order is affected by it

### Requirement: The winning arbitration key is persisted per target

For each target, the arbitration key of the currently materialized operation SHALL be persisted. An operation whose key is lower than the persisted key for its target SHALL be rejected without modifying that target.

#### Scenario: A late Notice does not clobber newer state

- **WHEN** an operation arrives after a higher-keyed operation for the same target has already been materialized
- **THEN** the target is left unchanged

#### Scenario: Incremental folding matches a full re-fold

- **WHEN** the graph is rebuilt by folding every Notice from an empty state
- **THEN** the resulting graph and persisted keys are identical to those produced by incremental folding of the same Notices

### Requirement: The fold is order-independent

The graph produced by folding a set of Notices SHALL depend only on the set, not on the order in which the Notices were delivered, batched, or applied.

#### Scenario: Permuted delivery order

- **WHEN** the same set of Notices is delivered to two devices in different permutations
- **THEN** both devices converge on an identical Mark/Terrain graph

#### Scenario: Reconvergence after offline divergence

- **WHEN** two devices edit the graph while unable to reach each other, then both sync
- **THEN** both converge on an identical Mark/Terrain graph

### Requirement: Notices are authored into a per-Landmark Track

Notices SHALL be authored with parents drawn from the heads of the Track associated with the Landmark the operation concerns, so that a Landmark's history occupies one Track. An operation concerning two Landmarks SHALL be authored into a Track chosen deterministically from the operation's content, so that every device makes the same choice.

#### Scenario: Successive edits to one Landmark share a Track

- **WHEN** several operations concerning the same Landmark are authored on one device
- **THEN** their Notices belong to the same Track

#### Scenario: Divergence resupplies only affected Landmarks

- **WHEN** one Landmark has diverged between two peers and others have not
- **THEN** resupply is requested only for the Track carrying the diverged Landmark

#### Scenario: A cross-Landmark operation is placed identically everywhere

- **WHEN** two devices independently determine the Track for an operation concerning two Landmarks
- **THEN** both choose the same Track

A cross-Landmark operation SHALL be authored with parents drawn from a single Track's heads. Naming the heads of both Landmarks' Tracks as parents SHALL NOT be used to place one Notice in two Tracks: a Notice belongs to exactly one Track, and naming a Track's head as a parent clears that head, leaving the other Track headless and fusing the two.

#### Scenario: Cross-Landmark authoring does not fuse Tracks

- **WHEN** an operation concerning two Landmarks is authored
- **THEN** its parents come from one Track's heads only, and the other Landmark's Track retains its own head

### Requirement: Per-Mark history is derived from operation targets, not from Tracks

Any history view of a Mark or Landmark SHALL be derived by filtering folded operations on their target. It SHALL NOT be derived by reading the Notices of a Track.

A Track is a transport partition, not a semantic history: an operation spanning two Landmarks appears in only one of their Tracks, and Tracks fuse over a Territory's lifetime, so Track membership does not correspond to Landmark identity.

#### Scenario: History of a Landmark touched by a cross-Landmark operation

- **WHEN** a history view is requested for a Landmark whose block was moved in from another Landmark
- **THEN** the moving operation appears in that history, regardless of which Track carries its Notice

#### Scenario: Track inspection is not a product surface

- **WHEN** a user-facing history, change log, or "what changed while offline" view is built
- **THEN** it reads the target-filtered operation view, and no user-facing view depends on Track membership

### Requirement: The Mark-to-Track index never affects graph state

The index mapping a Mark to its Track SHALL be consulted only when choosing the parents of a Notice being authored. It SHALL NOT be read while folding. Loss, staleness, or corruption of the index SHALL NOT change the graph the fold produces.

#### Scenario: Folding with no index present

- **WHEN** the graph is folded with the index absent or empty
- **THEN** the resulting Mark/Terrain graph is identical to the graph folded with an intact index

#### Scenario: A stale index degrades partitioning only

- **WHEN** the index no longer reflects a Landmark's Track and new operations for that Landmark begin a second Track
- **THEN** the Landmark's state remains correct
- **AND** the only consequence is that its history resupplies as two units instead of one

### Requirement: Operations referencing a not-yet-created Mark are deferred, never dropped

An operation referencing a Mark whose creation has not been folded SHALL be retained in a deferred set keyed by the missing Mark, and SHALL NOT be applied or discarded. When the missing Mark is created by a later fold step, its deferred operations SHALL be released and applied in arbitration-key order. The count of deferred operations SHALL be observable.

#### Scenario: An operation arrives before the Mark it references

- **WHEN** an operation referencing a Mark is folded before that Mark's creation
- **THEN** the operation is deferred and no row is written for the missing Mark

#### Scenario: The missing Mark arrives

- **WHEN** the creation of a previously missing Mark is folded
- **THEN** the operations deferred on it are applied in arbitration-key order
- **AND** the resulting state matches what folding in causal order would have produced

#### Scenario: A reference never resolves

- **WHEN** a referenced Mark's creation is never received
- **THEN** the dependent operations remain deferred rather than partially applied
- **AND** their count is reported alongside sync status

### Requirement: Log growth is measured

Each sync SHALL report the Territory's Notice count and the on-disk size of the hold. These values SHALL be reported as part of normal operation, not only under a diagnostic mode.

#### Scenario: Growth is visible after a sync

- **WHEN** a sync completes
- **THEN** the Notice count and hold size are recorded
