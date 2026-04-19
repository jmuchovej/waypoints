import type { ContentKind, InlineMarkKind, MarkID } from "@waypoints/domain";
import { MarkIDDataKey } from "#imports";

export interface BlockElement extends HTMLElement {
  dataset: DOMStringMap & {
    markId: MarkID;
    depth: number;
  };
}

export interface MarkElement extends HTMLElement {
  dataset: DOMStringMap & {
    type: ContentKind | string;
  };
}

export interface InlineElement extends HTMLElement {
  dataset: DOMStringMap & {
    inline: InlineMarkKind;
  };
}

export function getBlock(id: MarkID): BlockElement | null {
  return document.querySelector<BlockElement>(`[${MarkIDDataKey}="${id}"]`);
}

export function getBlocks(): NodeListOf<BlockElement> {
  return document.querySelectorAll<BlockElement>(`[${MarkIDDataKey}]`);
}

export function getMarkContent(el: BlockElement): BlockElement | MarkElement {
  return el.querySelector<MarkElement>("[data-type]") ?? el;
}

export function getClosestBlock(event: MouseEvent): BlockElement | null;
export function getClosestBlock(element: HTMLElement): BlockElement | null;
export function getClosestBlock(e: HTMLElement | MouseEvent): BlockElement | null {
  if (e instanceof MouseEvent) {
    return getClosestBlock(e.target as HTMLElement);
  }

  return e.closest<HTMLElement>(`[${MarkIDDataKey}]`) as BlockElement;
}

export function getMarkID(mark: BlockElement): MarkID;
export function getMarkID(html: HTMLElement): MarkID;
export function getMarkID(html: HTMLElement | null): MarkID | undefined;
export function getMarkID(el: BlockElement | HTMLElement | null): MarkID | undefined {
  return el?.getAttribute(MarkIDDataKey) as MarkID;
}

export function isClosestTargetMark(e: MouseEvent, markID: MarkID): boolean {
  const el = getClosestBlock(e);
  return getMarkID(el) !== markID;
}
