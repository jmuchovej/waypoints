export const HANDLE_HEIGHT = 24; // h-6 = 1.5rem
export const GUTTER_PX = 64; // 4rem — how far left of the block the handle stays visible

/** Compute the vertical center of the first text line relative to the mark element's top edge.
 *  For zero-height content (e.g. dividers), falls back to centering on the mark's full height. */
export function firstLineCenter(markEl: HTMLElement): number {
  const contentEl = markEl.querySelector<HTMLElement>("[data-type]") ?? markEl;

  // Dividers / decorative blocks have no intrinsic line-height — center on the mark itself.
  if (contentEl.clientHeight === 0) {
    return markEl.clientHeight / 2;
  }

  const lineHeight = parseFloat(getComputedStyle(contentEl).lineHeight);
  const paddingTop = parseFloat(getComputedStyle(markEl).paddingTop) || 0;
  return paddingTop + (Number.isFinite(lineHeight) ? lineHeight : HANDLE_HEIGHT) / 2;
}

/** Check whether a pointer position falls within the gutter zone of a mark element. */
export function isInGutter(markEl: HTMLElement, event: PointerEvent): boolean {
  const { clientX, clientY } = event;
  const markRect = markEl.getBoundingClientRect();
  const inVerticalBand = clientY >= markRect.top && clientY <= markRect.bottom;
  const inGutter = clientX >= markRect.left - GUTTER_PX && clientX < markRect.left;
  return inVerticalBand && inGutter;
}

/** Compute the handle's top/left offsets relative to a container element. */
export function handlePosition(
  markEl: HTMLElement,
  containerEl: HTMLElement,
): { top: number; left: number } {
  const containerRect = containerEl.getBoundingClientRect();
  const markRect = markEl.getBoundingClientRect();
  const center = firstLineCenter(markEl);
  return {
    top: markRect.top - containerRect.top + center - HANDLE_HEIGHT / 2,
    left: markRect.left - containerRect.left,
  };
}
