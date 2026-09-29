## Context

See proposal.md — _Why_ for the motivation and the five bypassing paths.

The constraints that shape this design, all read from the current tree:

- `CommandHistory` is **per-landmark**: `EditorService` holds
  `HashMap<MarkID, CommandHistory>` keyed by `landmark_id`, and
  `history_entry()` lazily creates a bucket. Every mutation that records
  history therefore needs a landmark key.
- `CommandHistory::push` takes an `is_text_edit` flag. When set, ops merge into
  the previous group if the primary mark matches and less than
  `COALESCE_WINDOW_MS` (500ms) has elapsed. **Every existing call site passes
  `false`** — the flag is implemented but unused.
- `CommandHistory::undo` inverts each op and calls `Maneuver::apply`. So an op's
  `apply` arm must be able to run on the undo path, not just the forward path.
- Existing mutations write to the store _directly_ and then construct a
  `Maneuver` describing what they just did. The op and the write are
  independently authored, which is precisely how path 3 came to persist a change
  while recording nothing.
- `Maneuver::target()` has **zero callers in the tree**. It is exported from
  `cartograph::editor` for a future resolver.
- `FeatureID` is `FeatureID(Base36ID(String))` — `Clone`, not `Copy`.
- `Reading` derives `Debug, Clone, PartialEq, Serialize, Deserialize, Type` —
  no `Default`.
- `EditorHandle`'s methods take `&self` and lock a mutex around the
  `EditorService`, so a `&self` → `&mut self` change on the service does not
  propagate outward to the IPC layer.
- `Landmark.children` has **no writer and no reader** in production code.
  `add_child` / `with_children` (`mark_data.rs:373,377`) have zero callers;
  `projection::project_landmark` walks `terrain::get_child_edges` instead. No
  SQL, generated column, or materialized view references it, and no frontend
  source reads it.
- `RichReference` is used by **nothing except** `Landmark.children`.
  `ReferenceDisplayMode` is used independently by
  `chart::feature_kind::FeatureKind::Reference`, so it survives regardless.
- **No type in the tree sets `#[serde(deny_unknown_fields)]`.** Removing a field
  is therefore read-compatible with already-persisted JSON: a stored
  `"children"` key is silently ignored on deserialize.

## Goals / Non-Goals

**Goals:**

- Make "every editor mutation is op-shaped" a checkable invariant rather than a
  convention, and make it hold.
- Give `Landmark`'s two high-traffic fields per-field ops so concurrent edits to
  different fields do not clobber each other under inversion.
- Leave one authority for undo/redo rather than two.

**Non-Goals (design-level, beyond the proposal's scope boundaries):**

- Retrofitting the build-op-then-apply discipline (Decision 1) onto the nine
  mutations that already record history correctly. They work; converting them is
  a separate cleanup.
- Making `Maneuver::apply` validating or transactional. It stays a mechanical
  writer.
- Introducing a territory-level or global undo stack.

## Decisions

### 1. The four rewritten mutation bodies construct the op first, then apply it

New and rewritten mutation bodies follow: gather pre-state → validate →
construct the `Maneuver` → `op.apply(db).await?` → `history.push(...)`. They do
not call `store::query::mark::*` directly. The fifth bypassing path,
`MarkHandle::update_title`, does not get its own body — it becomes a shortcut
delegating into the editor's title path, so it inherits this shape for free.

_Why:_ the op and the database write become one expression instead of two, so
they cannot disagree. Path 3's bug — a write with no matching op — is not
expressible in this shape.

_Alternative rejected:_ keep the existing write-then-describe shape for
consistency with the other nine mutations. Rejected because consistency with a
shape that produced this bug is not a virtue. The inconsistency is temporary and
visible; a note in the spec records the intended direction.

_Alternative rejected:_ convert all twelve mutations now. Rejected as scope
creep — it would touch every editor test for no behavioural gain.

### 2. Validation stays in the mutation layer; `Maneuver::apply` stays mechanical

`update_mark_reading` validates the incoming `Reading` against the Chart's
`Feature` schema (`validate_reading`, plus `allowed_types` checks that fetch the
referenced marks). This validation runs once, in `mutation::update_mark_reading`,
before the op is constructed. `Maneuver::UpdateReading`'s `apply` arm performs
no validation.

_Why:_ `apply` also runs on the undo and redo paths. If it validated, undoing to
a previously-valid state could _fail_ after an intervening Chart edit narrowed a
feature's constraints — and it would fail mid-loop in `CommandHistory::undo`,
leaving the database partly reverted with the group already popped. An op that
was legal when recorded must remain applicable.

_Alternative rejected:_ validate inside `apply` and have `CommandHistory::undo`
roll back on error. Rejected — that needs transactional undo, which is a much
larger change and not what this one is about.

### 3. `Reading::Null` is the encoding for "no reading"

Today `update_mark_reading` interprets `Reading::Null` as "remove the entry"
(`landmark.readings.shift_remove`). `UpdateReading` keeps that: `old` is the
prior `Reading` or `Reading::Null` if the feature was unset, and the `apply` arm
removes the entry when `new` is `Null` and inserts otherwise.

_Why:_ it makes `invert()` a straight `old`/`new` swap with no `Option` layer,
and it matches the semantics already exposed across the IPC boundary.

Because `Reading` has no `Default`, the skipped field needs an explicit
constructor, following the existing `empty_rich_text` precedent in
`maneuver.rs`:

```rust
UpdateReading {
    mark_id: MarkID,
    feature_id: FeatureID,
    /// Local-only, for CommandHistory undo.
    #[serde(skip, default = "reading_null")]
    old: Reading,
    new: Reading,
}
```

`UpdateLandmarkTitle`'s `old: RichText` reuses `empty_rich_text` unchanged.

### 4. `ManeuverTarget` drops `Copy`, keeps `Clone`

`ManeuverTarget::Reading(MarkID, FeatureID)` cannot be `Copy` because
`FeatureID` wraps a `String`. `target()` becomes a `Clone`-returning accessor.

_Why this is cheap:_ `target()` has no callers in the tree. The cost is
hypothetical; the benefit — per-field conflict targets, the entire point of the
change — is not.

_Alternative rejected:_ target readings as `ManeuverTarget::Mark(MarkID)` and
keep `Copy`. That reintroduces exactly the conflation being removed: two
readings on one landmark would share a target.

_Alternative rejected:_ intern `FeatureID` to a `Copy` handle. Premature — it
would put an interner in `domain` to preserve a derive nobody depends on.

### 5. History key for title and reading edits is the landmark's own `MarkID`

`update_mark_title(id, ...)` and `update_mark_reading(id, ...)` already receive
the landmark's `MarkID` as `id`. That is the history key.

_Why:_ no new parameter crosses the typed IPC boundary, and the one frontend
caller (`src/app/pages/marks/[markID].vue`) is editing the page it is
displaying, so `id` is already the landmark whose history the user expects
Cmd+Z to walk.

_Alternative rejected:_ add an explicit `landmark_id` parameter for symmetry
with the block mutations. Rejected — it would change three IPC signatures to
pass a value that is always equal to `id`.

### 6. `create_landmark` records into the new landmark's own history bucket

A newly created landmark has no containing landmark, so its `CreateMark` op goes
into `histories[new_id]`.

_Why:_ it keeps the invariant free of exceptions. Every mutation lands in some
bucket; none is special-cased out of the log.

_Consequence, accepted:_ undoing immediately after creating a page, while
sitting on that page, soft-deletes the page the user is looking at. This is
bounded (the page is empty and brand-new) and reversible (redo restores it —
`apply_create_mark` already handles the restore-a-soft-deleted-mark case).

_Alternative rejected:_ apply the op but skip the `history.push`. Rejected —
that is the exact "persists but records nothing" hole this change exists to
close, reintroduced deliberately.

_Alternative rejected:_ a territory-scoped history bucket for landmark-level
ops. Rejected as new machinery for a single op; revisit if landmark deletion or
move ever needs the same treatment.

### 7. The backend becomes the single undo authority

`useBlockOrchestrator.ts`'s `onUndo`/`onRedo` currently consult the focused
block's Tiptap handle first (`handle.canUndo()` → `handle.undo()`) and only fall
through to `ipc.editor.undo` when that block's local stack is empty. That branch
is removed; both handlers go straight to the backend.

_Why this is required, not optional:_ once `update_mark_text` records
`UpdateRichText`, a text edit exists in _both_ stacks. Keeping the short-circuit
means Cmd+Z reverts the edit in Tiptap while the backend op stays queued, and a
later backend undo reverts it a second time. Worse, the per-block Tiptap stacks
drain before any backend op regardless of when those edits happened, so undo
does not follow the user's actual edit order across blocks. That ordering defect
is a second, independent contributor to the reported bug — the backend fix alone
does not resolve it.

`useInlineEditor.ts` already resyncs Tiptap from props when external changes
arrive (its comment names undo/redo explicitly), so the backend-driven path is
the one the component was built for.

_The decisive reason, beyond the double-undo bug:_ a per-block Tiptap stack is
**structurally incapable** of being correct once operations can arrive from
elsewhere. An edit folded in from another device has no ProseMirror steps behind
it, so Tiptap cannot see it, cannot invert it, and cannot order it against local
edits. Any design that leaves Tiptap authoritative for text undo has to be
dismantled again later; the backend stack is the only one that can ever hold the
complete picture. Confirmed 2026-07-30 as the reason to accept the frontend
scope now rather than defer it.

_Trade-off — smaller than it first appeared._ Two apparent losses were checked
against the code and neither is real:

- _Input-rule/autoformat grouping._ Block-type conversion runs through
  `update_mark` (`mutation.rs:96`), which pushes with `is_text_edit: false` and
  therefore always starts a new group. So Cmd+Z after typing `# ` reverts the
  conversion alone and restores the paragraph text — the same result
  ProseMirror's `undoInputRule` gives, from grouping policy that already exists.
- _Paste as one undo unit._ `CommandHistory::push` takes `ops: Vec<Maneuver>`,
  so one call is one group is one undo. Because the editor is one Tiptap
  instance **per block**, a multi-block paste is necessarily handled at app level
  and arrives as a single op batch — which the backend groups naturally. Per-block
  Tiptap stacks could never have grouped across blocks at all. This is a
  capability gained, not lost.

What genuinely changes is that text-undo granularity becomes
`CommandHistory`'s 500ms coalesce window. Note this is _new_ behaviour rather
than changed behaviour: `is_text_edit` is currently passed `false` at every call
site, so the coalescer has never run in production (see Decision 8). Worth
exercising deliberately. `TextPayload` is a tagged enum precisely so a finer
payload kind can be added later without breaking anything; that is not done here.

### 8. `update_mark_text` is the only path that pushes with `is_text_edit = true`

It is the only mutation whose ops should merge into a preceding group.

_Why:_ the flag exists and is correct for this case; every other path is
structural and must start a fresh group. Its `CursorState` uses
`offset: None` for both before and after, matching `update_mark` — the backend
does not know the caret position, and the frontend restores focus by block ID.

### 9. `Landmark.children` is deleted, not documented

The field, its two builder mutators, and the doc comment naming it as a
guaranteed field are removed. `RichReference` is kept.

_Why delete rather than annotate:_ the alternative on the table was to leave the
field and describe it as vestigial in the architecture reference. That is
accurate but unstable — a struct field that serializes into every landmark's
persisted JSON is a standing invitation for a future contributor (or a future
sync merge law) to treat it as meaningful. The failure mode is specifically bad
under sync: one device populating a field another ignores is a silent
divergence with no conflict to detect. Deleting it makes the invariant
structural instead of documentary.

_Why this is cheap:_ nothing reads it, nothing writes it, no SQL projects it, no
frontend consumes it. The removal is a compile-time-checked edit to one struct
plus a binding regeneration.

_Why `RichReference` survives the removal of its only user:_ it is early, not
dead. Terrain edges express containment between Marks, but a Mark referenced
_within_ a `Content` mark — a mention mid-paragraph, an inline card — is not
naturally an edge in that graph, and the display-mode question ("render this
reference as an avatar, a mention, a card") is real for exactly that case.
Today it is carried untyped as `InlineMark { kind: Mention | Link, data:
Option<Json> }`. `RichReference` is the typed shape that payload wants, and
`ReferenceDisplayMode` is already live via `FeatureKind::Reference`. Deleting it
would remove the answer and keep the question.

_Alternative rejected:_ delete `RichReference` too, as dead code. Rejected on
the above — dead-by-reference-count is not dead-by-intent, and the type is
cheap to carry.

_Alternative rejected:_ defer the removal to the sync change. Rejected — sync is
precisely the change that would be harmed by inheriting an ambiguous field, and
this change is the one whose investigation established the finding.

## Risks / Trade-offs

- **Coarser text undo than users have today** → Accepted; see Decision 7. The
  debounce already dominates the granularity, and correct ordering is worth
  more than fine granularity that reverts the wrong thing.
- **Undo-after-create-page deletes the visible page** → Accepted; see Decision 6. Bounded and redo-reversible. Flagged here so it is a known behaviour rather
  than a surprise regression.
- **`ManeuverTarget` losing `Copy` is a public-API break** → Zero in-tree
  callers; `cartograph` is not published. Mitigated by landing it in the same
  commit as the variants that force it.
- **`update_mark_title` / `update_mark_reading` going `&mut self` could cascade
  into lock-contention changes** → It does not: `EditorHandle` already takes
  `&self` and locks a mutex around the service, so the mutability change is
  contained. Verified against `crates/cartograph/src/editor/handle.rs`.
- **Two mutation styles coexist in `mutation.rs` after this lands** (build-then-
  apply for the four rewritten bodies, write-then-describe for nine) → Accepted, and recorded
  in the spec as the intended direction so the next person does not "fix" the
  new code back to the old shape.
- **`create_landmark` moving off `ChartHandle` changes a Tauri command's backing
  call** → `src-tauri/src/commands/chart.rs` keeps its signature;
  only the delegation target moves. No IPC or frontend change. The same holds
  for `mark_update_title` (`src-tauri/src/commands/mark.rs`) once
  `MarkHandle::update_title` delegates to the editor.
- **The `children` / Terrain duplication is resolved by deletion** → See
  Decision 9. Terrain `Child` edges become the only expression of containment,
  structurally rather than by convention. Residual: landmark rows persisted
  before this change still carry a `"children"` key in their JSON. It is inert
  (nothing deserializes it) and disappears the next time each row is written.
- **`Landmark` changing shape is a breaking change to a serialized type** →
  Read-compatible in both directions that matter: old rows deserialize (no
  `deny_unknown_fields`), and new rows simply omit the key. The break is at the
  Rust struct and TypeScript binding level, where the compiler and `just
typecheck` catch it. No in-tree caller constructs `children`.

## Migration Plan

No data migration.

`Maneuver` is not persisted — `CommandHistory` lives in memory on
`EditorService` and is cleared on territory switch (`clear_histories`). Adding
variants changes no stored bytes and no schema.

Removing `Landmark.children` does change the serialized shape of landmark rows,
but needs no migration either. Nothing in the tree sets
`#[serde(deny_unknown_fields)]`, so already-persisted landmark JSON carrying a
`"children"` key deserializes unchanged — the key is ignored. Newly written rows
omit it. The two forms coexist indefinitely without a rewrite pass; a backfill
would be cosmetic, and is not performed. No SQL, generated column, or
materialized view reads the key, so nothing downstream observes the difference.

Rollback is a revert. Reverting after landing restores the field as empty on
read for any row written in the interim — harmless, since no code path consumed
it before or after.

## Open Questions

None that affect the specs, the approach, or the task breakdown.

The `Landmark.children` question that this change previously carried as open is
resolved, acted on, and specified — see Decision 9. The containment invariant it
leaves behind is stated normatively in this change's new `data-model` capability
spec, with the `editor` spec restating the projection-path consequence it owns.
Nothing about it remains deferred.
