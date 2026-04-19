import {
  type MarkID,
  type RichText,
  type Content,
  paragraph,
  contentData,
  isNestableType,
  isIndentableType,
  isCompoundType,
} from "@waypoints/domain";
import type { RenderNode } from "@waypoints/cartograph";
import ipc from "~/ipc";
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
  try {
    const res = await ipc.editor.moveMarks(parentID, childIDs, afterID, landmarkID);
    return res.blocks;
  } catch (e) {
    log.error(`moveMarks(${pos}) failed:`, e);
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
    try {
      const result = await ipc.editor.updateText(markId, richText, landmarkID.value);
      if (stateVersion === versionAtCall) {
        blocks.value = result.blocks;
      }
    } catch (e) {
      log.error("updateMarkText failed:", e);
    }
  }

  async function onSplitBlock(markId: MarkID, offset: number) {
    await flushPendingUpdates();
    stateVersion++;
    try {
      const result = await ipc.editor.split(markId, offset, landmarkID.value);
      blocks.value = result.blocks;
      focus.focusBlock(result.new_mark_id, "start");
    } catch (e) {
      log.error("splitMark failed:", e);
    }
  }

  async function onMergeBlocks(targetId: MarkID, sourceId: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    try {
      const result = await ipc.editor.merge(targetId, sourceId, landmarkID.value);
      blocks.value = result.blocks;
      focus.focusBlock(targetId, "end");
    } catch (e) {
      log.error("mergeMarks failed:", e);
    }
  }

  async function onDeleteBlocks(markIds: MarkID[]) {
    await flushPendingUpdates();
    stateVersion++;
    try {
      const result = await ipc.editor.deleteMarks(markIds, landmarkID.value);
      blocks.value = result.blocks;
    } catch (e) {
      log.error("deleteMarks failed:", e);
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
    try {
      const result = await ipc.editor.undo(landmarkID.value);
      if (result) {
        blocks.value = result.blocks;
        if (result.cursor) {
          focus.focusBlock(result.cursor.block_id);
        }
      }
    } catch (e) {
      log.error("undo failed:", e);
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
    try {
      const result = await ipc.editor.redo(landmarkID.value);
      if (result) {
        blocks.value = result.blocks;
        if (result.cursor) {
          focus.focusBlock(result.cursor.block_id);
        }
      }
    } catch (e) {
      log.error("redo failed:", e);
    }
  }

  async function onCreateFirstBlock() {
    await flushPendingUpdates();
    stateVersion++;
    const id = crypto.randomUUID();
    try {
      await ipc.marks.create(id, landmarkID.value, paragraph(), new Date().toISOString(), null);
      blocks.value = await ipc.editor.projectLandmark(landmarkID.value, null);
      focus.focusBlock(id, "start");
    } catch (e) {
      log.error("markCreate (first block) failed:", e);
    }
  }

  async function onCreateBlockBefore(beforeId: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const siblings = getSiblings(beforeId);
    const idx = siblings.findIndex((b) => b.id === beforeId);
    if (idx > 0) {
      // Insert after the previous sibling (which places it before the target)
      const prevSibling = siblings[idx - 1]!;
      const parentId = prevSibling.parent_id ?? landmarkID.value;
      try {
        const result = await ipc.editor.createAfter(
          prevSibling.id,
          parentId,
          contentData(paragraph()),
          landmarkID.value,
        );
        blocks.value = result.blocks;
        focus.focusBlock(result.new_mark_id, "start");
      } catch (e) {
        log.error("createBlockBefore failed:", e);
      }
    } else {
      // Target is the first child — use marks.create with null afterId
      const block = blocks.value.find((b) => b.id === beforeId);
      const parentId = block?.parent_id ?? landmarkID.value;
      const id = crypto.randomUUID();
      try {
        await ipc.marks.create(id, parentId, paragraph(), new Date().toISOString(), null);
        blocks.value = await ipc.editor.projectLandmark(landmarkID.value, null);
        focus.focusBlock(id, "start");
      } catch (e) {
        log.error("createBlockBefore (first child) failed:", e);
      }
    }
  }

  async function onCreateBlockAfter(afterId: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const block = blocks.value.find((b) => b.id === afterId);
    const parentId = block?.parent_id ?? landmarkID.value;
    try {
      const result = await ipc.editor.createAfter(
        afterId,
        parentId,
        contentData(paragraph()),
        landmarkID.value,
      );
      blocks.value = result.blocks;
      focus.focusBlock(result.new_mark_id, "start");
    } catch (e) {
      log.error("createMarkAfter failed:", e);
    }
  }

  async function onIndentBlock(id: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const block = blocks.value.find((b) => b.id === id);
    if (block && !isIndentableType(block.data)) return;
    const siblings = getSiblings(id);
    const idx = siblings.findIndex((b) => b.id === id);
    if (idx <= 0) return;
    const prevSibling = siblings[idx - 1]!;
    if (!isNestableType(prevSibling.data)) return;
    try {
      const result = await ipc.editor.reparent(id, prevSibling.id, null, landmarkID.value);
      blocks.value = result.blocks;
      focus.focusBlock(id);
    } catch (e) {
      log.error("reparentMark (indent) failed:", e);
    }
  }

  async function onDedentBlock(id: MarkID) {
    await flushPendingUpdates();
    stateVersion++;
    const block = blocks.value.find((b) => b.id === id);
    if (!block || block.depth === 0) return;
    const parent = blocks.value.find((b) => b.id === block.parent_id);
    if (!parent) return;
    const grandparentId = parent.parent_id ?? landmarkID.value;
    try {
      const result = await ipc.editor.reparent(id, grandparentId, parent.id, landmarkID.value);
      blocks.value = result.blocks;
      focus.focusBlock(id);
    } catch (e) {
      log.error("reparentMark (dedent) failed:", e);
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

    if (isCompoundType(newContent.type)) {
      // Compound blocks: create the compound and reparent the current block
      // (or create an empty child if the block has no text).
      const block = blocks.value.find((b) => b.id === id);
      const parentId = block?.parent_id ?? landmarkID.value;
      const hasContent =
        block?.data.mark === "content" &&
        "rich_text" in block.data &&
        block.data.rich_text.segments.some((s: { text: string }) => s.text);

      try {
        const result = await ipc.editor.wrapInCompound(
          contentData(newContent),
          hasContent ? [id] : [],
          hasContent ? null : id,
          parentId,
          landmarkID.value,
        );
        blocks.value = result.blocks;

        if (hasContent) {
          // Focus the reparented block (now inside the compound)
          focus.focusBlock(id, "start");
        } else {
          // Delete the original empty block, focus the new child
          try {
            const deleteResult = await ipc.editor.deleteMarks([id], landmarkID.value);
            blocks.value = deleteResult.blocks;
          } catch {
            // Non-critical — compound was created successfully
          }
          focus.focusBlockDeferred(result.new_mark_id, "start");
        }
      } catch (e) {
        log.error("wrapInCompound failed:", e);
      }
    } else {
      try {
        const result = await ipc.editor.updateMark(id, newContent, landmarkID.value);
        focus.focusBlockDeferred(id, "start");
        blocks.value = result.blocks;
      } catch (e) {
        log.error("updateMark (changeType) failed:", e);
      }
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
    onCreateBlockBefore,
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
