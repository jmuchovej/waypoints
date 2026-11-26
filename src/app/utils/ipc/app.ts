// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import type { SelfUpdatePolicy, SelfUpdateStatus } from "@waypoints/cartograph";

import { invoke } from "./invoke";

/** The self-update setting, the cap the launch set, and what they resolve to. */
export async function selfUpdateStatus(): Promise<SelfUpdateStatus> {
  return invoke<SelfUpdateStatus>("self_update_status");
}

/** Stores the user's setting; the cap is the launcher's and never changes here. */
export async function selfUpdateSet(setting: SelfUpdatePolicy): Promise<SelfUpdateStatus> {
  return invoke<SelfUpdateStatus>("self_update_set", { setting });
}
