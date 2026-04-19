import type {
  Content,
  FeatureID,
  Mark,
  MarkData,
  MarkID,
  Reading,
  RichText,
} from "@waypoints/domain";
import type {
  CreateResult,
  MutationResult,
  RenderNode,
  SplitResult,
  UndoRedoResult,
} from "@waypoints/cartograph";
import { invoke } from "./invoke";

export async function getMark(markId: MarkID): Promise<Mark> {
  return invoke<Mark>("editor_get_mark", { markId });
}

export async function listMarks(): Promise<Mark[]> {
  return invoke<Mark[]>("editor_mark_list");
}

export async function projectLandmark(
  landmarkId: MarkID,
  maxDepth: number | null,
): Promise<RenderNode[]> {
  return invoke<RenderNode[]>("editor_project_landmark", { landmarkId, maxDepth });
}

export async function updateMark(
  id: MarkID,
  content: Content,
  landmarkId: MarkID,
): Promise<MutationResult> {
  return invoke<MutationResult>("editor_update_mark", { id, content, landmarkId });
}

export async function deleteMark(id: MarkID, landmarkId: MarkID): Promise<MutationResult> {
  return invoke<MutationResult>("editor_delete_mark", { id, landmarkId });
}

export async function moveMarks(
  parentId: MarkID,
  childIds: MarkID[],
  afterId: MarkID | null,
  landmarkId: MarkID,
): Promise<MutationResult> {
  return invoke<MutationResult>("editor_move_marks", { parentId, childIds, afterId, landmarkId });
}

export async function updateTitle(id: MarkID, title: RichText): Promise<void> {
  return invoke<void>("editor_update_mark_title", { id, title });
}

export async function updateReading(
  id: MarkID,
  featureId: FeatureID,
  reading: Reading,
): Promise<Mark> {
  return invoke<Mark>("editor_update_mark_reading", { id, featureId, reading });
}

export async function wrapInCompound(
  compoundData: MarkData,
  childIds: MarkID[],
  afterId: MarkID | null,
  parentId: MarkID,
  landmarkId: MarkID,
): Promise<CreateResult> {
  return invoke<CreateResult>("editor_wrap_in_compound", {
    compoundData,
    childIds,
    afterId,
    parentId,
    landmarkId,
  });
}

export async function createAfter(
  afterId: MarkID,
  parentId: MarkID,
  data: MarkData,
  landmarkId: MarkID,
): Promise<CreateResult> {
  return invoke<CreateResult>("editor_create_mark_after", { afterId, parentId, data, landmarkId });
}

export async function updateText(
  markId: MarkID,
  richText: RichText,
  landmarkId: MarkID,
): Promise<MutationResult> {
  return invoke<MutationResult>("editor_update_mark_text", { markId, richText, landmarkId });
}

export async function split(
  markId: MarkID,
  offset: number,
  landmarkId: MarkID,
): Promise<SplitResult> {
  return invoke<SplitResult>("editor_split_mark", { markId, offset, landmarkId });
}

export async function merge(
  targetId: MarkID,
  sourceId: MarkID,
  landmarkId: MarkID,
): Promise<MutationResult> {
  return invoke<MutationResult>("editor_merge_marks", { targetId, sourceId, landmarkId });
}

export async function deleteMarks(markIds: MarkID[], landmarkId: MarkID): Promise<MutationResult> {
  return invoke<MutationResult>("editor_delete_marks", { markIds, landmarkId });
}

export async function reparent(
  markId: MarkID,
  newParentId: MarkID,
  afterId: MarkID | null,
  landmarkId: MarkID,
): Promise<MutationResult> {
  return invoke<MutationResult>("editor_reparent_mark", {
    markId,
    newParentId,
    afterId,
    landmarkId,
  });
}

export async function undo(landmarkId: MarkID): Promise<UndoRedoResult | null> {
  return invoke<UndoRedoResult | null>("editor_undo", { landmarkId });
}

export async function redo(landmarkId: MarkID): Promise<UndoRedoResult | null> {
  return invoke<UndoRedoResult | null>("editor_redo", { landmarkId });
}
