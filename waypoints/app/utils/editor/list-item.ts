import { ContentType, isContent, type ContentData, type MarkData } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/cartograph";

function isIterable(
  data: MarkData,
): data is ContentData<typeof ContentType.ListItem | typeof ContentType.EnumItem> {
  return isContent(data, [ContentType.ListItem, ContentType.EnumItem]);
}

/**
 * Compute the sequential 1-based index for a list-item or enum-item within
 * its parent+depth group.  Returns `undefined` for non-list blocks.
 *
 * The counter resets whenever a non-list block at the same depth/parent
 * interrupts the run.
 */
export function indexFor(
  block: RenderNode,
  _blockIndex: number,
  blocks: RenderNode[],
): number | undefined {
  if (!isIterable(block.data)) return undefined;

  // Find the block's actual index in the full blocks array
  const actualIndex = blocks.findIndex((b) => b.id === block.id);
  if (actualIndex < 0) return undefined;

  let count = 0;
  for (let i = 0; i <= actualIndex; i++) {
    const b = blocks[i]!;
    const isSameGroup = b.parent_id === block.parent_id && b.depth === block.depth;

    if (isSameGroup) {
      if (isIterable(b.data)) {
        count++;
      } else if (i < actualIndex) {
        // Non-list block at same depth/parent resets the counter
        count = 0;
      }
    }
  }

  return count;
}
