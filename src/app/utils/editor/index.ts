export * from "./elements";
export * from "./floating-handle";
export * from "./list-item";
export * from "./registries";

import type { RenderNode } from "@waypoints/cartograph";
import type { ContentData, ContentKind, LandmarkData } from "@waypoints/domain";

export { ContentType, InlineKind } from "@waypoints/domain";

/** A RenderNode whose `data` is narrowed to a specific content type (or union of types). */
export type ContentNode<K extends ContentKind = ContentKind> = Omit<RenderNode, "data"> & {
  data: ContentData<K>;
};

/** A RenderNode whose `data` is narrowed to landmark type (or union of types). */
export type LandmarkNode = Omit<RenderNode, "data"> & {
  data: LandmarkData;
};
