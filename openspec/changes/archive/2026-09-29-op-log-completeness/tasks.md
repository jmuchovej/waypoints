Every task below must land with `prek` hooks green (clippy, cargo-check,
cargo-test, treefmt). Never hand-format — `treefmt` owns formatting.
Rust editor tests run through the existing `rstest` + `store::test_utils::temp_db()`
fixture in `crates/cartograph/src/editor/tests/mod.rs`, against the real engine.
No test asserts SQL strings. Frontend tests run under `vitest`.

## 1. Operation vocabulary

- [x] 1.1 In `crates/cartograph/src/editor/maneuver.rs`, add
      `Maneuver::UpdateLandmarkTitle { mark_id, old, new }` with
      `#[serde(skip, default = "empty_rich_text")] old: RichText` and
      `new: RichText`.
- [x] 1.2 Add a `reading_null()` helper alongside `empty_rich_text()`, then add
      `Maneuver::UpdateReading { mark_id, feature_id, old, new }` with
      `#[serde(skip, default = "reading_null")] old: Reading` and `new: Reading`.
      (`Reading` has no `Default`, hence the named constructor.)
- [x] 1.3 Add `ManeuverTarget::Title(MarkID)` and
      `ManeuverTarget::Reading(MarkID, FeatureID)`. Drop the `Copy` derive from
      `ManeuverTarget` (`FeatureID` wraps a `String`) and keep `Clone`; adjust
      `Maneuver::target()` to clone where it previously copied. Confirm no caller
      breaks — `target()` currently has none in-tree.
- [x] 1.4 Extend `Maneuver::target()` with arms mapping `UpdateLandmarkTitle` to
      `Title(mark_id)` and `UpdateReading` to `Reading(mark_id, feature_id)`.
- [x] 1.5 Extend `Maneuver::invert()`: both new variants invert by swapping
      `old` and `new`.
- [x] 1.6 Extend `Maneuver::apply()` with two mechanical writers — one setting
      `Landmark.title`, one inserting the reading or removing it when `new` is
      `Reading::Null`. Both error if the target Mark is missing or is not a
      landmark. Neither performs schema validation (design.md, Decision 2).
- [ ] ~~1.7 Rewrite the `Maneuver` doc comments at `maneuver.rs:25` and `:91`~~
      **STRUCK.** Based on a false premise: those comments were read as describing a
      _retired_ wire protocol. They are not stale — they are **anticipatory**, and
      they describe exactly the design the sync change implements (`Envelope` is an
      `outpost-protocol` type, which is why it does not appear in `cartograph`
      today). Rewriting them here would be churn the next change reverts. **Leave
      both comments as written.**
- [x] 1.8 Add unit coverage in `crates/cartograph/src/editor/tests/` that each
      new variant round-trips through `invert()` (double inversion is the
      identity) and that `target()` yields distinct targets for two features of
      one landmark, and for a title versus a reading on one landmark.

## 2. Landmark title path

- [x] 2.1 Rewrite `mutation::update_mark_title` to take
      `history: &mut CommandHistory`, read the pre-state title, reject non-landmark
      Marks before touching anything, construct `UpdateLandmarkTitle`, call
      `op.apply(db)`, then `history.push(vec![op], id, cursor, cursor, false)` with
      `CursorState { block_id: id, offset: None }`. It must no longer call
      `mark::update_data` directly.
- [x] 2.2 Change `EditorService::update_mark_title` to `&mut self` and resolve
      its history bucket with `history_entry(&mut self.histories, id)` — the
      landmark's own ID is the key (design.md, Decision 5). Update
      `EditorHandle::update_mark_title` if the borrow needs it; its `&self` +
      mutex-lock shape should already suffice.
- [x] 2.3 Add tests: undo restores the previous title; two successive title
      edits undo one at a time; a title edit leaves `chart`, `icon`, and
      `readings` untouched; setting a title on a content Mark is rejected and
      pushes nothing onto the history.
- [x] 2.4 Confirm `src-tauri/src/commands/editor.rs` and
      `tooling/xtask/src/tasks/dev_server/commands/editor.rs` still compile
      unchanged (no IPC signature change is expected), and run `just typegen`
      if the generated bindings move.
- [x] 2.5 Rewire `MarkHandle::update_title` in `crates/cartograph/src/mark/handle.rs`
      to delegate to the editor's title path — it is a shortcut for updating a
      Landmark's title and must no longer read via `store::query::mark::get` and
      write `MarkData` directly. Keep its public signature so the Tauri command
      `mark_update_title` (`src-tauri/src/commands/mark.rs`), the dev-server
      xtask route, and `src/app/utils/ipc/marks.ts` are unaffected. Add a test
      that a title edit through `MarkHandle::update_title` is undoable.
      **Superseded in review:** the delegate shim kept a second write path, so
      `MarkHandle::update_title` was removed outright; the Tauri command and
      xtask route call `rt.editor().update_mark_title` directly. The IPC
      surface (`mark_update_title`, `ipc.marks.updateTitle`) is unchanged.

## 3. Reading path

- [x] 3.1 Rewrite `mutation::update_mark_reading` to take
      `history: &mut CommandHistory`. Keep the existing validation in place and
      ahead of op construction: chart lookup, `feature.kind.validate_reading`, and
      the `allowed_types` reference check. Then capture the prior reading (or
      `Reading::Null` when unset), construct `UpdateReading`, `op.apply(db)`,
      `history.push(...)`, and re-fetch the Mark for the return value.
- [x] 3.2 Change `EditorService::update_mark_reading` to `&mut self`, keyed on
      the landmark's own ID. Mirror in `EditorHandle` if required.
- [x] 3.3 Add tests: undo restores a changed reading; undo restores a cleared
      reading; undo removes a newly set reading on a previously unset feature; two
      features edited in sequence undo independently; an invalid reading is
      rejected with readings unchanged and nothing pushed; a reference reading
      outside `allowed_types` is rejected with nothing pushed.
- [x] 3.4 Add a test that undo succeeds after a Chart's constraints are narrowed
      such that the earlier reading would no longer validate — this is what pins
      Decision 2 (apply does not revalidate).

## 4. Text path

- [x] 4.1 Rewrite `mutation::update_mark_text` to use the `history` parameter it
      already receives (drop the `_` prefix). Read the pre-state `RichText`,
      construct `Maneuver::UpdateRichText { mark_id, old, new: TextPayload::Snapshot { .. } }`,
      `op.apply(db)`, then push with `is_text_edit = true` — the only call site
      that sets the flag. Cursor before and after are
      `CursorState { block_id: mark_id, offset: None }`.
- [x] 4.2 Delete the doc comment claiming the function "does NOT record in the
      undo history" and replace it with one describing the coalescing behaviour.
- [x] 4.3 Add tests: undo restores the previous text; several updates to one
      Mark inside the 500ms coalesce window undo as a single step; a structural
      mutation between two text updates prevents coalescing so one undo reverses
      only the later update.

## 5. Landmark creation

- [x] 5.1 Add `mutation::create_landmark(db, history, chart_id) -> EditorResult<Mark>`:
      mint the `MarkID`, build the `Landmark` via its builder, construct
      `Maneuver::CreateMark`, `op.apply(db)` (its `apply_create_mark` helper
      already handles the restore-a-soft-deleted-mark case that redo needs), push
      to history, and return the Mark.
- [x] 5.2 Add `EditorService::create_landmark`, resolving the history bucket
      with the newly minted `MarkID` as the key (design.md, Decision 6). Expose it
      on `EditorHandle`.
- [x] 5.3 Rewrite `ChartHandle::create_landmark` in
      `crates/cartograph/src/chart/handle.rs` to delegate to the editor. It must no
      longer call `store::query::mark::insert`. Keep its public signature and its
      `tracing::info!` line so `src-tauri/src/commands/chart.rs` is
      unaffected.
      **Superseded in review:** the delegate shim kept a second write path, so
      `ChartHandle::create_landmark` was removed outright; the Tauri command
      and xtask route call `rt.editor().create_landmark` directly, and the
      `tracing::info!` line moved into `EditorService::create_landmark`. The
      IPC surface (`landmark_create`) is unchanged.
- [x] 5.4 Add tests: creating a landmark persists it and records a `CreateMark`
      in that landmark's history; undo soft-deletes it; redo restores it with the
      same `MarkID`.

## 6. Single undo authority (frontend)

- [x] 6.1 In `src/app/composables/editor/useBlockOrchestrator.ts`, remove
      the Tiptap-first short-circuit from `onUndo` and `onRedo` (the
      `focus.getHandle(blockId)` / `canUndo()` / `canRedo()` branches). Both
      handlers go straight to `flushPendingUpdates()` then `ipc.editor.undo` /
      `ipc.editor.redo`.
- [x] 6.2 Remove the now-unused `undo` / `redo` / `canUndo` / `canRedo` members
      from the block-handle contract in
      `src/app/composables/editor/useBlockFocus.ts` and their implementations
      in `useInlineEditor.ts`, **only if** nothing else consumes them — grep first
      and leave them in place with a note if anything does. Leave
      `useInlineEditor.ts`'s external-change resync untouched; the backend path
      depends on it.
- [x] 6.3 Update the affected specs:
      `src/app/composables/editor/useBlockKeymap.test.ts` and
      `useBlockFocus.test.ts` both stub `undo`/`redo` on the handle mock. Adjust
      them to assert that Cmd+Z reaches the backend path.
- [x] 6.4 Run `pnpm exec vitest` unit tests and `just typecheck`.

## 7. Landmark schema cleanup

Independent of sections 1–6; may land first. Do it in one commit so the struct
change and the regenerated bindings never disagree.

- [x] 7.1 In `crates/domain/src/mark/mark_data.rs`, delete the `children` field
      from `Landmark` (line ~382) together with the `via_mutators` block
      declaring `add_child` and `with_children` (~371-381). Drop the now-unused
      `RichReference` import from the `use super::{...}` line at the top **only
      if** nothing else in the file references it.
- [x] 7.2 Fix the `Landmark` doc comment (~line 345), which names `children` as
      a guaranteed top-level field. Guaranteed fields are `id`, `title`, `icon`.
      Add a line stating that a Landmark's content is expressed by Terrain
      `Child` edges, not by a field on the Landmark, so the next reader does not
      re-add one.
- [x] 7.3 **Keep `RichReference`.** Add a doc note to
      `crates/domain/src/mark/rich_reference.rs` recording why it has no use
      site: it is the typed payload intended for inline Mark references inside
      `Content` marks (`InlineMark { kind: Mention | Link, data }`), which
      currently carry untyped JSON. Correct its existing comment at line ~36,
      which cites "top-level Thing fields like children" — that use site no
      longer exists. Do **not** remove `ReferenceDisplayMode`; it is live via
      `chart::feature_kind::FeatureKind::Reference`.
- [x] 7.4 Run `just typegen` and commit the regenerated
      `crates/domain/typescript/types.g.ts` and
      `crates/cartograph/typescript/types.g.ts`. Confirm `RichReference` is still
      exported in both (it is registered independently in
      `crates/domain/bin/typegen.rs`) and that `children` is gone from the
      `Landmark` type in both.
- [x] 7.5 Update `.agents/skills/waypoints-architecture/references/architecture.md`:
      §5's "Children" subsection and its vestigial callout, the guaranteed-fields
      sentence, the struct sketch (~line 162), and §7's containment note
      (~line 335). All currently say "vestigial but present"; they become
      "removed". State plainly that Terrain `Child` edges are the only
      expression of containment and ordering.
- [x] 7.6 Add a test that a round-trip of a `Landmark` through
      serialize/deserialize is unaffected by an extra `"children"` key in the
      input JSON — this pins the no-migration claim (design.md, Migration Plan)
      against a future `deny_unknown_fields` being added somewhere upstream.

## 8. Verification

- [x] 8.1 Sweep `crates/cartograph/src/editor/mutation.rs`,
      `crates/cartograph/src/chart/handle.rs`, and
      `crates/cartograph/src/mark/handle.rs` for any remaining direct
      `store::query::mark::*` or `store::query::terrain::*` write that is not
      paired with a `history.push`. Confirm none remain outside the nine
      pre-existing mutations that already record correctly.
- [x] 8.2 Confirm no `history: &mut CommandHistory` parameter in the editor is
      underscore-prefixed or otherwise unused.
- [ ] ~~8.3 Confirm `Envelope` appears nowhere in `crates/cartograph`.~~
      **STRUCK.** This would enshrine a false invariant: the sync change makes
      `cartograph` reference `outpost-protocol`'s `Envelope` deliberately. Asserting
      its absence here would have to be deleted one change later.
- [x] 8.4 Confirm `Landmark.children` appears nowhere in the tree — not in
      `crates/`, not in the generated `types.g.ts` files, not in the
      architecture reference except as a historical note.
- [x] 8.5 Run the full editor suite (`just cartograph::test`; note the root
      `just test` runs only the frontend vitest suite) plus `just typecheck`,
      and confirm `prek` is green across clippy, cargo-check, cargo-test, and
      treefmt.
