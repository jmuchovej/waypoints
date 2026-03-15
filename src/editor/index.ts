import type { ContentData, ContentKind, LandmarkData } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/editor";

/** A RenderNode whose `data` is narrowed to a specific content type (or union of types). */
export type ContentNode<K extends ContentKind = ContentKind> = Omit<RenderNode, "data"> & {
  data: ContentData<K>;
};

/** A RenderNode whose `data` is narrowed to a landmark. */
export type LandmarkNode = Omit<RenderNode, "data"> & {
  data: LandmarkData;
};
