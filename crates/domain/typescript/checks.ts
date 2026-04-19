import type { Content, Feature, FeatureKind, InlineMark, Mark, MarkData } from "./types.g";

export type MarkKind = MarkData["mark"];

export type InlineMarkKind = InlineMark["kind"];
export type ContentKind = Content["type"];
export type ContentData<K extends ContentKind = ContentKind> = Extract<
  MarkData,
  { mark: "content"; type: K }
>;
export type LandmarkData = Extract<MarkData, { mark: "landmark" }>;
export type LandmarkMark = Omit<Mark, "data"> & { data: LandmarkData };

export type ContentMark<K extends ContentKind = ContentKind> = Omit<Mark, "data"> & {
  data: ContentData<K>;
};

export type FeatureKindType = FeatureKind["type"];
export type FeatureKindData<K extends FeatureKindType = FeatureKindType> = Extract<
  FeatureKind,
  { type: K }
>;
export type FeatureData<K extends FeatureKindType = FeatureKindType> = Omit<Feature, "kind"> & {
  kind: FeatureKindData<K>;
};

const compoundTypes = ["blockquote", "callout"] as const;
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

const nestableTypes = ["paragraph", "list-item", "enum-item"] as const;
export type NestableType = (typeof nestableTypes)[number];

export function isNestableType(kind: ContentKind): kind is NestableType;
export function isNestableType(data: MarkData): data is ContentData<NestableType>;
export function isNestableType(input: MarkData | ContentKind): input is ContentData<NestableType> {
  if (typeof input === "string") {
    return nestableTypes.includes(input as NestableType);
  }
  return isContent(
    input,
    nestableTypes.map((t) => t as ContentKind),
  );
}

const nonIndentableTypes = ["heading-1", "heading-2", "heading-3"] as const;
export type NonIndentableType = (typeof nonIndentableTypes)[number];

export function isIndentableType(kind: ContentKind): boolean;
export function isIndentableType(data: MarkData): boolean;
export function isIndentableType(input: MarkData | ContentKind): boolean {
  if (typeof input === "string") {
    return !nonIndentableTypes.includes(input as NonIndentableType);
  }
  return isContent(input) && !nonIndentableTypes.includes(input.type as NonIndentableType);
}

const nonFocusableTypes = [...compoundTypes, "table", "divider"] as const;
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
