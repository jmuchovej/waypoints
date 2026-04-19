import type { ContentData, ContentKind, LandmarkData } from "@waypoints/domain";
import type { RenderNode } from "@waypoints/cartograph";

export { CommandMenu } from "./tiptap/extensions/command-menu";
export { ContentType, InlineKind } from "@waypoints/domain";

export { default as DocumentBody } from "./components/DocumentBody.vue";
export { default as RichTextEditor } from "./components/RichTextEditor.vue";

export * from "./composables";
export * from "./utils/elements";

/** A RenderNode whose `data` is narrowed to a specific content type (or union of types). */
export type ContentNode<K extends ContentKind = ContentKind> = Omit<RenderNode, "data"> & {
  data: ContentData<K>;
};

/** A RenderNode whose `data` is narrowed to landmark type (or union of types). */
export type LandmarkNode = Omit<RenderNode, "data"> & {
  data: LandmarkData;
};
