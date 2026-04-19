import type { Content, Mark, MarkID, RichText } from "@waypoints/domain";
import { invoke } from "./invoke";

export async function list(): Promise<Mark[]> {
  return invoke<Mark[]>("mark_list");
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
