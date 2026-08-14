import type { Chart, ChartID, Mark } from "@waypoints/domain";

import { invoke } from "./invoke";

export async function list(): Promise<Chart[]> {
  return invoke<Chart[]>("chart_list");
}

export async function get(chartId: ChartID): Promise<Chart | null> {
  return invoke<Chart | null>("chart_get", { chartId });
}

export async function createLandmark(chartId: ChartID): Promise<Mark> {
  return invoke<Mark>("landmark_create", { chartId });
}
