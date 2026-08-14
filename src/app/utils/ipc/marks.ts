import type {
  ChartID,
  Content,
  ContentMark,
  LandmarkMark,
  Mark,
  MarkID,
  MarkKind,
  RichText,
} from "@waypoints/domain";

import { invoke } from "./invoke";

interface LandmarkListOptions {
  allowedTypes?: ChartID[];
}

export function list(): Promise<Mark[]>;
export function list(kind: "landmark", options?: LandmarkListOptions): Promise<LandmarkMark[]>;
export function list(kind: "content"): Promise<ContentMark[]>;
export function list(kind?: MarkKind, options?: LandmarkListOptions): Promise<Mark[]> {
  return invoke<Mark[]>("mark_list", {
    prefix: kind ?? null,
    allowedTypes: options?.allowedTypes ?? null,
  });
}

export async function create(
  id: string,
  parentId: string,
  content: Content,
  createdAt: string,
  afterId: string | null,
): Promise<void> {
  return invoke<void>("mark_create", { id, parentId, content, createdAt, afterId });
}

export async function update(id: string, content: Content): Promise<void> {
  return invoke<void>("mark_update", { id, content });
}

export async function remove(id: string): Promise<void> {
  return invoke<void>("mark_delete", { id });
}

export async function move(
  parentId: string,
  childId: string,
  afterId: string | null,
): Promise<void> {
  return invoke<void>("mark_move", { parentId, childId, afterId });
}

export async function updateTitle(id: MarkID, title: RichText): Promise<void> {
  return invoke<void>("mark_update_title", { id, title });
}
