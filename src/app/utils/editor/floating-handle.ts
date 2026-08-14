export const HANDLE_HEIGHT = 24; // h-6 = 1.5rem
export const GUTTER_PX = 64; // 4rem — how far left of the block the handle stays visible

/** Compute the vertical center of the handle anchor relative to the mark element's top edge.
 *
 *  For compound blocks (callouts, blockquotes) aligns with the prefix/icon.
 *  For code blocks, aligns with the header bar.
 *  For zero-height content (e.g. dividers), centers on the mark's full height.
 *  Otherwise, centers on the first text line. */
export function firstLineCenter(markEl: HTMLElement): number {
  // Callout/blockquote: align with the prefix element (icon / accent bar)
  const prefix = markEl.querySelector<HTMLElement>(".prefix");
  if (prefix) {
    const markRect = markEl.getBoundingClientRect();
    const prefixRect = prefix.getBoundingClientRect();
    return prefixRect.top - markRect.top + prefixRect.height / 2;
  }

  // Code block: align with the header bar
  const codeHeader = markEl.querySelector<HTMLElement>(".code-header");
  if (codeHeader) {
    const markRect = markEl.getBoundingClientRect();
    const headerRect = codeHeader.getBoundingClientRect();
    return headerRect.top - markRect.top + headerRect.height / 2;
  }

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
