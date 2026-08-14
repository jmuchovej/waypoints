## Why

Undo/redo in the block editor has never behaved the way users expect. The
diagnosis is structural, not incidental: `cartograph`'s editor is built on the
premise that every mutation is _op-shaped_ — it produces one or more
`Maneuver` values recorded in `CommandHistory`, and `CommandHistory` inverts
those ops to undo. Four high-traffic mutation paths do not honour that premise.
They write to the database directly and record nothing, so the edits they make
are not merely hard to undo, they are **structurally un-undoable**: there is no
record for the history to invert.

The four paths, all verified in the current tree:

| #   | Path                                                                 | How it bypasses                                                                                                      |
| --- | -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| 1   | `crates/cartograph/src/editor/mutation.rs:205` `update_mark_title`   | No `history` parameter at all; calls `mark::update_data` directly.                                                   |
| 2   | `crates/cartograph/src/editor/mutation.rs:223` `update_mark_reading` | No `history` parameter; loads the mark, mutates one reading, writes the whole `MarkData` back.                       |
| 3   | `crates/cartograph/src/editor/mutation.rs:353` `update_mark_text`    | Takes `_history` (underscore-prefixed, unused). Its own doc comment states it "does NOT record in the undo history". |
| 4   | `crates/cartograph/src/chart/handle.rs:32` `create_landmark`         | Skips the editor entirely; calls `store::query::mark::insert` directly.                                              |

Together these cover the most common things a user does: rename a page, fill in
a field, and type. Closing them is what makes the op log complete, and a
complete op log is the precondition for undo/redo being correct at all.

## What Changes

**New op vocabulary.** `Maneuver::UpdateMarkData` carries a whole `MarkData`
and targets `ManeuverTarget::Mark(MarkID)`. That coarse granularity is _correct_
for `MarkData::Content` — a paragraph's text is genuinely one value, and text
already has its own finer `UpdateRichText` op. It is _wrong_ for
`MarkData::Landmark`, where a single op conflates four independent fields
(`chart`, `title`, `icon`, `readings`). Two title edits and a
reading edit interleaved through one whole-value op cannot be inverted
independently. So:

- Add `Maneuver::UpdateLandmarkTitle { mark_id, old, new }`, targeting a new
  `ManeuverTarget::Title(MarkID)`.
- Add `Maneuver::UpdateReading { mark_id, feature_id, old, new }`, targeting a
  new `ManeuverTarget::Reading(MarkID, FeatureID)`.

`icon` and `chart` stay coarse under `UpdateMarkData` — low traffic, and
splitting them now would be speculative.

**Schema cleanup.** `Landmark` carries four fields rather than five because this
change **removes `Landmark.children: Vec<RichReference>`** outright. Containment
and ordering are carried entirely by Terrain `Child` edges and their `order`
fractional index; `children` is never written by any code path
(`add_child`/`with_children` in `domain::mark::mark_data` have zero callers in
the tree) and never read by `projection::project_landmark`. Leaving a field that
one device could populate and another ignores is a silent-divergence hazard the
sync change would inherit, and describing it as "vestigial" only invites the
next reader to re-litigate it. **BREAKING** for the `Landmark` struct shape and
its generated TypeScript bindings; no in-tree caller is affected.

`RichReference` itself is **retained**. It is not dead — it is ahead of its use
site. A Mark referenced _inside_ a `Content` mark (a mention, an inline card, an
embed) cannot practicably be a Terrain edge, and today that case is carried by
`InlineMark { kind: Mention | Link, data: Option<Json> }` with an untyped JSON
payload. `RichReference` is the typed shape that payload wants. Removing it
would delete the answer to a problem the codebase still has.

**Plumbing, not vocabulary.** Paths 3 and 4 map onto existing variants
(`UpdateRichText` and `CreateMark`). They need routing, not new op kinds.

- `update_mark_text` records `UpdateRichText` against the `history` it is
  already handed.
- `create_landmark` moves onto the editor and emits `CreateMark`.
  **BREAKING** for the `ChartHandle::create_landmark` call site.

**Signature changes.** `EditorService::update_mark_title` and
`update_mark_reading` currently take `&self`. Recording history requires
`&mut self`. The `EditorHandle` wrappers already hold the service behind a
mutex, so this does not reach the typed IPC surface. **BREAKING** for direct
`EditorService` callers.

**Single undo authority.** `useBlockOrchestrator.ts` currently tries Tiptap's
per-block local history first and only falls through to the backend when that
block's local stack is empty. Once text edits are in the backend log, keeping
that branch produces double-undo and out-of-order undo. The branch is removed
so the backend log is the sole authority. See design.md for the granularity
trade-off this implies.

## Capabilities

### New Capabilities

- `editor`: block mutation, undo/redo, projection, and selection semantics for
  `cartograph`'s editor and the `app` surface that drives it. This change seeds
  the capability with the op-log completeness invariant and the `Maneuver` /
  `ManeuverTarget` vocabulary; it does not attempt to describe the capability
  exhaustively.
- `domain`: the Mark data model — what a Landmark is, what it carries, and how
  containment is expressed. This change seeds it with one invariant only:
  a Landmark's content is expressed solely by Terrain child edges, and Landmark
  data carries no child list. It makes no attempt to describe Charts, Readings,
  Beacons, or the rest of the model; those emerge from the changes that touch
  them.

### Modified Capabilities

None. `openspec/specs/` is empty — specs emerge from differentials rather than
being backfilled, so this change creates the `editor` and `domain` specs rather
than amending existing ones.

## Non-goals

- **No sync, no wire format.** No serialization of `Maneuver` into any sync
  payload, no notice authoring, no network code. `Maneuver` already derives
  `Serialize`/`Deserialize` and that stays as-is; nothing in this change reads
  or writes those bytes. This is purely local correctness. Sync is a separate
  change.
- **No merge law, no conflict resolution, no clocks.** `ManeuverTarget` gains
  two variants; how a resolver would consult them is out of scope.
- **No splitting of `icon` or `chart` out of `UpdateMarkData`.**
- **No new home for what `children` gestured at.** The field is deleted, not
  replaced. Inline references inside `Content` marks stay on
  `InlineMark.data`'s untyped JSON; typing that payload with `RichReference` is
  a separate change.
- **No character-level text ops.** `TextPayload` keeps `Snapshot` as its only
  variant.
- **No rewrite of `Maneuver`'s wire-type doc comments.** An earlier draft of
  this proposal treated the comments at `maneuver.rs:25` and `:59` — ops
  travelling between devices "inside `Envelope`s" — as describing a retired
  protocol, on the reasoning that no `Envelope` type exists in `cartograph`.
  That reasoning was wrong. `Envelope` is an `outpost-protocol` type
  (`crates/outpost-protocol/src/envelope.rs:78`), and the comments are
  anticipatory rather than stale: the in-flight `sync-mark-graph` change cites
  them as existing groundwork it builds on (its `design.md:8`) and declares a
  dependency on this change. Rewriting them here would be churn the next change
  reverts. Both comments are left exactly as written. See tasks 1.7 and 8.3,
  struck for this reason.

## Resolved during this change

`Landmark.children` versus Terrain `Child` edges was previously carried as an
open question — two apparent authorities for one fact. Investigation found the
situation was not symmetric at all: **no code in `cartograph` or `store` ever
writes `Landmark.children`.** The only writers are the builder mutators in
`domain::mark::mark_data`, which nothing calls. Ordering and containment are
carried entirely by Terrain `Child` edges with fractional indices
(`terrain::get_child_edges`, `mutation::compute_order`), and
`projection::project_landmark` reads only those.

That settles the authority question — Terrain wins — and this change acts on it
by deleting the field rather than annotating it. See _What Changes → Schema
cleanup_ and design.md, Decision 9.

**The invariant it leaves behind is specified here, not deferred.**
_Containment and ordering of a Landmark's content are expressed solely by
Terrain `Child` edges_ is a data-model rule rather than an editor rule, so it
is stated normatively in a new `domain` capability spec (`specs/domain/spec.md`)
along with the decision record behind it. The `editor` spec restates the
projection-path consequence it owns — projection reads edges and nothing else —
and points at `domain` for the rest.

The split between requirement and decision is about where a reader looks, not
about what survives — archived changes retain their artifacts, and those are
kept in version control. But `sync` promotes requirement blocks into
`openspec/specs/`, which is what someone consults to learn the system's current
contract. An invariant recorded only as a Decisions entry is found by reading
the change that produced it, which assumes the reader already knows it exists.
Stating it as a requirement puts it where it will be encountered without being
sought. The rationale stays in the Decisions log, where the reasoning belongs.

The architecture reference (§5, §7) is updated to match, but is documentation
rather than a spec.

## Impact

**Code**

- `crates/cartograph/src/editor/maneuver.rs` — two new `Maneuver` variants, two
  new `ManeuverTarget` variants, `target()` / `invert()` / `apply()` arms, doc
  comments. `ManeuverTarget` loses its `Copy` derive (`FeatureID` wraps
  `Base36ID(String)`); `target()` becomes a clone-returning method.
- `crates/cartograph/src/editor/mutation.rs` — `update_mark_title` and
  `update_mark_reading` gain a `history` parameter; `update_mark_text` uses the
  one it has; a new `create_landmark`.
- `crates/cartograph/src/editor/service.rs` — `&self` → `&mut self` on two
  methods; a new `create_landmark` entry point.
- `crates/cartograph/src/editor/handle.rs` — mirror the service signatures.
- `crates/cartograph/src/chart/handle.rs` — `create_landmark` delegates to the
  editor instead of calling `store::query::mark::insert`.
- `waypoints/app/composables/editor/useBlockOrchestrator.ts` — remove the
  Tiptap-first undo/redo short-circuit.
- `crates/domain/src/mark/mark_data.rs` — delete the `children` field, its
  `add_child` / `with_children` builder mutators, and the guaranteed-fields doc
  comment that names it. `RichReference` stays; its module gains a doc note
  recording the use site it is waiting on.
- `crates/domain/typescript/types.g.ts` and
  `crates/cartograph/typescript/types.g.ts` — regenerated via `just typegen`.
  `RichReference` remains exported (it is registered in `domain/bin/typegen.rs`
  independently); `Landmark.children` disappears from both.
- `.agents/skills/waypoints-architecture/references/architecture.md` — §5 and §7
  currently describe `children` as vestigial-but-present. They become
  "removed".

**Unchanged surfaces**

- Typed IPC: `waypoints/src-tauri/src/commands/{editor,chart}.rs`,
  `crates/server/src/commands/editor.rs`, and
  `waypoints/app/utils/ipc/editor.ts` keep their current argument shapes. The
  history key for a title or reading edit is the landmark's own `MarkID`, which
  callers already pass as `id`, so no new parameter crosses the boundary.
- `store` is untouched. `domain` is touched only by the `children` removal
  above — no query, migration, or schema-definition change. No SQL references
  `$.children`, and no generated column or materialized view depends on it.
- Frontend sources are untouched by the schema cleanup: no `.vue` or `.ts` file
  reads `landmark.children`. Only the generated bindings change.

**Tests**

- `crates/cartograph/src/editor/tests/{mutations,undo_redo}.rs` gain coverage
  for the four paths. The existing `rstest` + `store::test_utils::temp_db()`
  fixture runs against the real engine; no SQL strings are asserted.
- `waypoints/app/composables/editor/useBlockKeymap.test.ts` and
  `useBlockFocus.test.ts` reference the Tiptap undo handle and need updating
  alongside the orchestrator change.
