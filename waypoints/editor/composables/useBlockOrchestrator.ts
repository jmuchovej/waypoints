import {
  type MarkID,
  type RichText,
  type Content,
  paragraph,
  contentData,
  isNestableType,
  isIndentableType,
} from "@waypoints/domain";
import type { RenderNode } from "@waypoints/editor";
import { commands } from "~/bindings.g";
import { useBlockFocus } from "./useBlockFocus";
import { useBlockSelection } from "./useBlockSelection";
import { type BlockPosition, useBlockDragDrop } from "./useBlockDragDrop";
import { type Ref, provide } from "vue";
import { createLogger } from "~/utils/logger";

const log = createLogger("editor");

export const BlockOrchestratorKey = "block-orchestrator";

async function moveBlocks(
  parentID: string,
  childIDs: MarkID[],
  afterID: string | null,
  landmarkID: string,
  pos: BlockPosition,
): Promise<RenderNode[] | undefined> {
  const res = await commands.editorMoveMarks(parentID, childIDs, afterID, landmarkID);
  switch (res.status) {
    case "ok":
      return res.data.blocks;
    default:
      log.error(`moveMarks(${pos}) failed: `, res.error);
      return undefined;
  }
}

export async function doMoveBlocks(
  landmarkID: string,
  srcIDs: MarkID[],
  dstID: MarkID,
  pos: BlockPosition,
  dropDepth: number,
  blocks: RenderNode[],
): Promise<RenderNode[] | undefined> {
  if (pos === "inside") {
    // dstID IS the compound — append after the last existing child
    const children = blocks.filter((b) => b.parent_id === dstID);
    const afterID = children.length > 0 ? children[children.length - 1]!.id : null;
    return await moveBlocks(dstID, srcIDs, afterID, landmarkID, pos);
  }

  const dst = blocks.find((b) => b.id === dstID);
  if (!dst) return undefined;

  if (dropDepth > dst.depth && isNestableType(dst.data)) {
    // Indent: target becomes the parent, append after its last child
    const children = blocks.filter((b) => b.parent_id === dstID);
    const afterID = children.length > 0 ? children[children.length - 1]!.id : null;
    return await moveBlocks(dstID, srcIDs, afterID, landmarkID, pos);
  }

  if (dropDepth < dst.depth) {
    // Dedent: walk up ancestors until we find the right scope
    let ancestor: RenderNode | undefined = dst;
    while (ancestor && ancestor.depth > dropDepth) {
      ancestor = blocks.find((b) => b.id === ancestor!.parent_id);
    }
    if (!ancestor) {
      // Reached root level
      return await moveBlocks(landmarkID, srcIDs, dstID, landmarkID, pos);
    }
    const parentID = ancestor.parent_id ?? landmarkID;
    return await moveBlocks(parentID, srcIDs, ancestor.id, landmarkID, pos);
  }

  // Same depth — sibling placement (original logic)
  const parentID = dst.parent_id ?? landmarkID;

  switch (pos) {
    case "after":
      return await moveBlocks(parentID, srcIDs, dstID, landmarkID, pos);
    case "before": {
      const siblings = blocks.filter((b) => b.parent_id === dst.parent_id && b.depth === dst.depth);
      const idx = siblings.findIndex((b) => b.id === dstID);
      const afterID = idx > 0 ? siblings[idx - 1]!.id : null;
      return await moveBlocks(parentID, srcIDs, afterID, landmarkID, pos);
    }
  }
}

export function useBlockOrchestrator(
  landmarkID: Ref<string>,
  blocks: Ref<RenderNode[]>,
  containerEl: Ref<HTMLElement | null>,
) {
  const focus = useBlockFocus(blocks);
  const dragDrop = useBlockDragDrop(blocks, containerEl, async (srcIDs, dstID, pos, depth) => {
    await flushPendingUpdates();
    stateVersion++;
    const res = await doMoveBlocks(landmarkID.value, srcIDs, dstID, pos, depth, blocks.value);
    if (res) blocks.value = res;
  });
  const selection = useBlockSelection(blocks, focus);

  // ── State versioning (prevents stale text-update responses from overwriting structural mutations) ──
  let stateVersion = 0;

  // ── Sibling resolution ──────────────────────────────────────────────

  function getSiblings(blockId: MarkID): RenderNode[] {
    const block = blocks.value.find((b) => b.id === blockId);
    if (!block) return [];
    return blocks.value.filter((b) => b.parent_id === block.parent_id && b.depth === block.depth);
  }

  function getBlockIndex(blockId: MarkID): number {
    return blocks.value.findIndex((b) => b.id === blockId);
  }

  function getBlockAbove(blockId: MarkID): RenderNode | null {
    const idx = getBlockIndex(blockId);
    return idx > 0 ? blocks.value[idx - 1]! : null;
  }

  function getBlockBelow(blockId: MarkID): RenderNode | null {
    const idx = getBlockIndex(blockId);
    return idx >= 0 ? blocks.value[idx + 1]! : null;
  }

  // ── Flush pending updates before structural mutations ───────────────

  async function flushPendingUpdates() {
    if (focus.blockID.value) {
      const handle = focus.getHandle(focus.blockID.value);
      await handle?.flush();
    }
  }

  // ── Structural command wrappers ─────────────────────────────────────

  async function onUpdateBlockText(markId: MarkID, richText: RichText) {
    const versionAtCall = stateVersion;
    const result = await commands.editorUpdateMarkText(markId, richText, landmarkID.value);
    if (result.status === "ok" && stateVersion === versionAtCall) {
      blocks.value = result.data.blocks;
    } else if (result.status === "error") {
      log.error("updateMarkText failed:", result.error);
    }
  }

  async function onSplitBlock(markId: MarkID, offset: number) {
    await flushPendingUpdates();
    stateVersion++;
    const result = await commands.editorSplitMark(markId, offset, landmarkID.value);
    if (result.status === "ok") {
      blocks.value = result.data.blocks;
      focus.focusBlock(result.data.new_mark_id, "start");
    } else {
      log.error("splitMark failed:", result.error);
    }
  }

  async function onMergeBlocks(targetId: MarkID, sourceId: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const result = await commands.editorMergeMarks(targetId, sourceId, landmarkID.value);
    if (result.status === "ok") {
      blocks.value = result.data.blocks;
      focus.focusBlock(targetId, "end");
    } else {
      log.error("mergeMarks failed:", result.error);
    }
  }

  async function onDeleteBlocks(markIds: MarkID[]) {
    await flushPendingUpdates();
    stateVersion++;
    const result = await commands.editorDeleteMarks(markIds, landmarkID.value);
    if (result.status === "ok") {
      blocks.value = result.data.blocks;
    } else {
      log.error("deleteMarks failed:", result.error);
    }
  }

  async function onUndo() {
    // If the focused block's editor has local undo entries, let Tiptap handle it.
    const blockId = focus.blockID.value;
    if (blockId) {
      const handle = focus.getHandle(blockId);
      if (handle?.canUndo()) {
        handle.undo();
        return;
      }
    }

    // Otherwise, fall through to backend structural undo.
    await flushPendingUpdates();
    stateVersion++;
    const result = await commands.editorUndo(landmarkID.value);
    if (result.status === "ok" && result.data) {
      blocks.value = result.data.blocks;
      if (result.data.cursor) {
        focus.focusBlock(result.data.cursor.block_id);
      }
    } else if (result.status === "error") {
      log.error("undo failed:", result.error);
    }
  }

  async function onRedo() {
    // If the focused block's editor has local redo entries, let Tiptap handle it.
    const blockId = focus.blockID.value;
    if (blockId) {
      const handle = focus.getHandle(blockId);
      if (handle?.canRedo()) {
        handle.redo();
        return;
      }
    }

    // Otherwise, fall through to backend structural redo.
    await flushPendingUpdates();
    stateVersion++;
    const result = await commands.editorRedo(landmarkID.value);
    if (result.status === "ok" && result.data) {
      blocks.value = result.data.blocks;
      if (result.data.cursor) {
        focus.focusBlock(result.data.cursor.block_id);
      }
    } else if (result.status === "error") {
      log.error("redo failed:", result.error);
    }
  }

  async function onCreateFirstBlock() {
    await flushPendingUpdates();
    stateVersion++;
    const id = crypto.randomUUID();
    const createResult = await commands.markCreate(
      id,
      landmarkID.value,
      paragraph(),
      new Date().toISOString(),
      null,
    );
    if (createResult.status === "ok") {
      // Re-project to get the updated block list
      const projResult = await commands.editorProjectLandmark(landmarkID.value, null);
      if (projResult.status === "ok") {
        blocks.value = projResult.data;
        focus.focusBlock(id, "start");
      }
    } else {
      log.error("markCreate (first block) failed:", createResult.error);
    }
  }

  async function onCreateBlockAfter(afterId: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const block = blocks.value.find((b) => b.id === afterId);
    const parentId = block?.parent_id ?? landmarkID.value;
    const result = await commands.editorCreateMarkAfter(
      afterId,
      parentId,
      contentData(paragraph()),
      landmarkID.value,
    );
    if (result.status === "ok") {
      blocks.value = result.data.blocks;
      focus.focusBlock(result.data.new_mark_id, "start");
    } else {
      log.error("createMarkAfter failed:", result.error);
    }
  }

  async function onIndentBlock(id: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const block = blocks.value.find((b) => b.id === id);
    if (block && !isIndentableType(block.data)) return; // headings can't be indented
    const siblings = getSiblings(id);
    const idx = siblings.findIndex((b) => b.id === id);
    if (idx <= 0) return; // no previous sibling to nest under
    const prevSibling = siblings[idx - 1]!;
    if (!isNestableType(prevSibling.data)) return; // only nestable types can accept children
    const result = await commands.editorReparentMark(id, prevSibling.id, null, landmarkID.value);
    if (result.status === "ok") {
      blocks.value = result.data.blocks;
      focus.focusBlock(id);
    } else {
      log.error("reparentMark (indent) failed:", result.error);
    }
  }

  async function onDedentBlock(id: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const block = blocks.value.find((b) => b.id === id);
    if (!block || block.depth === 0) return; // already at root
    const parent = blocks.value.find((b) => b.id === block.parent_id);
    if (!parent) return;
    const grandparentId = parent.parent_id ?? landmarkID.value;
    const result = await commands.editorReparentMark(
      id,
      grandparentId,
      parent.id,
      landmarkID.value,
    );
    if (result.status === "ok") {
      blocks.value = result.data.blocks;
      focus.focusBlock(id);
    } else {
      log.error("reparentMark (dedent) failed:", result.error);
    }
  }

  async function onMoveBlockUp(id: MarkID) {
    const siblings = getSiblings(id);
    const idx = siblings.findIndex((b) => b.id === id);
    if (idx <= 0) return;
    await onMoveBlock(id, siblings[idx - 1]!.id, "before");
    focus.focusBlock(id);
  }

  async function onMoveBlockDown(id: MarkID) {
    const siblings = getSiblings(id);
    const idx = siblings.findIndex((b) => b.id === id);
    if (idx < 0 || idx >= siblings.length - 1) return;
    await onMoveBlock(id, siblings[idx + 1]!.id, "after");
    focus.focusBlock(id);
  }

  async function onMoveBlock(srcID: MarkID, dstID: MarkID, position: BlockPosition) {
    await flushPendingUpdates();
    stateVersion++;

    const srcBlock = blocks.value.find((b) => b.id === srcID);
    const res = await doMoveBlocks(
      landmarkID.value,
      [srcID],
      dstID,
      position,
      srcBlock?.depth ?? 0,
      blocks.value,
    );
    if (res) {
      blocks.value = res;
    }
  }

  async function onChangeBlockType(id: MarkID, newContent: Content) {
    await flushPendingUpdates();
    stateVersion++;
    const result = await commands.editorUpdateMark(id, newContent, landmarkID.value);
    if (result.status === "ok") {
      focus.focusBlockDeferred(id, "start");
      blocks.value = result.data.blocks;
    } else {
      log.error("updateMark (changeType) failed:", result.error);
    }
  }

  // ── Provide context to children ─────────────────────────────────────

  const context = {
    focus,
    selection,
    dragDrop,
    blocks,
    landmarkID,
    getSiblings,
    getBlockIndex,
    getBlockAbove,
    getBlockBelow,
    onUpdateBlockText,
    onSplitBlock,
    onMergeBlocks,
    onDeleteBlocks,
    onUndo,
    onRedo,
    onCreateFirstBlock,
    onCreateBlockAfter,
    onIndentBlock,
    onDedentBlock,
    onMoveBlockUp,
    onMoveBlockDown,
    onMoveBlock,
    onChangeBlockType,
    /** Set by DocumentBody when it has an external navigation target above. */
    onExitTop: undefined as (() => void) | undefined,
  };

  provide(BlockOrchestratorKey, context);

  return context;
}

export type OrchestratorContext = ReturnType<typeof useBlockOrchestrator>;
