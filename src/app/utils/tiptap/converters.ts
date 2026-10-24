import type { JSONContent } from "@tiptap/core";
import {
  InlineKind,
  type InlineMark,
  type RichText,
  type TextMarkKind,
  type TextSegment,
} from "@waypoints/domain";

/** A single TipTap inline mark (bold, link, etc.). */
type TipTapMark = NonNullable<JSONContent["marks"]>[number];

/** Convert a domain InlineMark to a TipTap mark object. */
export function toTipTapMark(mark: InlineMark): TipTapMark {
  if (mark.kind === InlineKind.Link && typeof mark.data === "string") {
    return { type: mark.kind, attrs: { href: mark.data } };
  }
  // A mention's data is the target id as a bare string — what `toInlineMark`
  // writes below, what `cartograph::editor::projection` reads, and what the
  // importer produces. Reading `.id` off it yielded `undefined` for every
  // mention.
  if (mark.kind === InlineKind.Mention && typeof mark.data === "string") {
    return { type: mark.kind, attrs: { id: mark.data } };
  }
  if (
    (mark.kind === InlineKind.Highlight || mark.kind === InlineKind.Color) &&
    typeof mark.data === "string"
  ) {
    return { type: mark.kind, attrs: { color: mark.data } };
  }
  if (mark.kind === InlineKind.Math && mark.data && typeof mark.data === "object") {
    return { type: mark.kind, attrs: { engine: (mark.data as any).engine ?? "typst" } };
  }
  return { type: mark.kind };
}

/** Convert a TipTap mark object back to a domain InlineMark. */
export function toInlineMark(mark: TipTapMark): InlineMark {
  if (mark.type === InlineKind.Link && mark.attrs?.href) {
    return { kind: mark.type, data: mark.attrs.href };
  }
  if (mark.type === InlineKind.Mention && mark.attrs?.id) {
    return { kind: InlineKind.Mention, data: mark.attrs.id };
  }
  if ((mark.type === InlineKind.Highlight || mark.type === InlineKind.Color) && mark.attrs?.color) {
    return { kind: mark.type as TextMarkKind, data: mark.attrs.color };
  }
  if (mark.type === InlineKind.Math) {
    return { kind: InlineKind.Math, data: { engine: mark.attrs?.engine ?? "typst" } };
  }
  return { kind: mark.type as TextMarkKind };
}

/** Convert domain RichText to an array of TipTap text nodes (paragraph content). */
export function toTipTap(rt: RichText): JSONContent[] {
  return rt.segments
    .filter((s: TextSegment) => s.text)
    .map((s: TextSegment) => ({
      type: "text" as const,
      text: s.text,
      ...(s.marks && s.marks.length > 0 ? { marks: s.marks.map(toTipTapMark) } : {}),
    }));
}

/** Convert TipTap paragraph content nodes back to domain RichText. */
export function toDomain(nodes: JSONContent[]): RichText {
  const segments: TextSegment[] = [];
  for (const node of nodes) {
    if (node.type === "text" && node.text) {
      segments.push({
        text: node.text,
        marks: (node.marks ?? []).map(toInlineMark),
      });
    } else if (node.type === "emoji" && node.attrs?.emoji) {
      segments.push({ text: node.attrs.emoji, marks: [] });
    }
  }
  return {
    segments,
    plain_text: segments.map((s) => s.text).join(""),
  };
}
