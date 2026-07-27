---
name: waypoints-architecture
description: >
  Reference documentation for Waypoints' data architecture. Consult this skill
  before evaluating any feature proposal, assessing architectural fit, or
  reasoning about what the system can and cannot represent. Covers the Mark,
  Chart, Landmark, Terrain, and Beacon data model at the level needed to make
  feature decisions. This is a living document that evolves with the codebase.
---

# Waypoints Architecture Reference

This skill provides architectural reference material for Waypoints. Read the
reference document before making any claims about what the system supports,
what constraints exist, or what infrastructure is missing.

## When to read the reference

- Before evaluating whether a feature fits the Mark architecture
- Before claiming something is or isn't possible in the current system
- When reasoning about data model constraints (e.g., edge identity, FeatureKind limitations)
- When assessing what infrastructure is missing for a proposed feature
- Before assuming anything about sync — what replicates, how ordering works, and what is actually reachable (§11)

> Two traps this document exists to prevent: assuming the Terrain multigraph is
> unimplemented (it is implemented — the PK is a per-edge `TerrainID`), and
> assuming note content syncs (it does not — only identity and pairing do).

## Reference

The full architecture document is at:

→ `references/architecture.md`

Read it in full on first use. On subsequent invocations, consult the relevant
sections — particularly **§10 (What the Architecture Cannot Yet Do)** for
current blockers and gaps.

## Key sections

| Section                   | Use when...                                                                 |
| ------------------------- | --------------------------------------------------------------------------- |
| §2 Marks                  | Reasoning about the atomic unit, MarkID stability, the `extra` field        |
| §3 Content                | Evaluating new block types; understanding the closed enum constraint        |
| §4 RichText and TextMarks | Anything involving inline formatting, offset-based ranges                   |
| §5 Landmarks              | Evaluating typed data, Chart conformance, children vs. Terrain              |
| §6 Charts                 | Evaluating type definitions, FeatureKind limitations, built-in Charts       |
| §7 Terrain                | Evaluating graph relationships, edge identity, the implemented multigraph   |
| §10 Cannot Yet Do         | **Always check this** before proposing work that assumes infrastructure     |
| §11 Sync (outpost)        | Anything involving multi-device, Notices, Tracks, ordering, or reachability |
