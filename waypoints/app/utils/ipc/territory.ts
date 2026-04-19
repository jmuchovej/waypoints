import type { Territory } from "@waypoints/cartograph";
import { invoke } from "./invoke";

export async function list(): Promise<Territory[]> {
  return invoke<Territory[]>("territory_list");
}

export async function getActive(): Promise<Territory | null> {
  return invoke<Territory | null>("territory_get_active");
}

export async function create(name: string): Promise<Territory> {
  return invoke<Territory>("territory_create", { name });
}

export async function open(
  id: string | null,
  path?: string | null,
  name?: string | null,
): Promise<Territory> {
  return invoke<Territory>("territory_open", { id, path: path ?? null, name: name ?? null });
}

export async function close(): Promise<void> {
  return invoke<void>("territory_close");
}

export async function remove(id: string, deleteData: boolean): Promise<void> {
  return invoke<void>("territory_remove", { id, deleteData });
}
