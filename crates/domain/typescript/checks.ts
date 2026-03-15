import type { Content, InlineMark, MarkData } from "./types.g";

export type InlineMarkKind = InlineMark["kind"];
export type ContentKind = Content["type"];
export type ContentData<K extends ContentKind = ContentKind> = Extract<
  MarkData,
  { mark: "content"; type: K }
>;
export type LandmarkData = Extract<MarkData, { mark: "landmark" }>;

const compoundTypes = ["blockquote", "callout", "table"] as const;
export type CompoundType = (typeof compoundTypes)[number];

export function isCompoundType(kind: ContentKind): kind is CompoundType;
export function isCompoundType(data: MarkData): data is ContentData<CompoundType>;
export function isCompoundType(input: MarkData | ContentKind): input is ContentData<CompoundType> {
  if (typeof input === "string") {
    return compoundTypes.includes(input as CompoundType);
  }
  return isContent(
    input,
    compoundTypes.map((t) => t as ContentKind),
  );
}

const nonFocusableTypes = [...compoundTypes, "divider"] as const;
export type NonFocusableType = (typeof nonFocusableTypes)[number];

export type FocusableType = Exclude<ContentKind, NonFocusableType>;

function isNonFocusableType(input: ContentKind): input is NonFocusableType {
  return nonFocusableTypes.includes(input as NonFocusableType);
}

export function isFocusableType(type: ContentKind): type is FocusableType;
export function isFocusableType(data: MarkData): data is ContentData<FocusableType>;
export function isFocusableType(input: MarkData | ContentKind): input is ContentData<CompoundType> {
  if (typeof input === "string") {
    return !isNonFocusableType(input);
  }
  return isContent(input) && !isNonFocusableType(input.type);
}

export function getContentKind(data: MarkData): ContentKind | undefined {
  if (isLandmark(data)) return undefined;
  return data.type;
}

export function getLandmarkKind(data: MarkData): string | undefined {
  if (isContent(data)) return undefined;
  return data.chart;
}

export function isContent<K extends ContentKind>(data: MarkData, kind: K): data is ContentData<K>;
export function isContent<K extends ContentKind>(
  data: MarkData,
  kinds: K[],
): data is ContentData<K>;
export function isContent(data: MarkData): data is ContentData;
export function isContent(data: MarkData, kind?: ContentKind[] | ContentKind): data is ContentData {
  if (data.mark !== "content") return false;

  if (kind === undefined) return true;

  const kinds = Array.isArray(kind) ? kind : [kind];
  return kinds.some((kind) => data.type === kind);
}

export function isLandmark(data: MarkData): data is LandmarkData;
export function isLandmark(data: MarkData): boolean {
  return data.mark === "landmark";
}
