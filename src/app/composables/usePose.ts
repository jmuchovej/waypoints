// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import type { ComputedRef } from "vue";

/** The query keys a page honours, each with the values it accepts. */
export type PoseKeys = Record<string, readonly string[]>;

/** The keys a link set to an accepted value; every other key is absent. */
export type Pose<K extends PoseKeys> = { readonly [P in keyof K]?: K[P][number] };

/**
 * Presentation state a page reads from its route's query — which panel is
 * open, which item is selected — so a link can open the page posed.
 *
 * A page declares the keys it honours and the values each accepts. Anything
 * else in the query (undeclared keys, unaccepted values, repeated keys) is
 * ignored, and nothing is written back to the route. A pose is presentation
 * only: no key may create, edit or delete anything (see the deep-link plugin).
 *
 * @example
 *   const pose = usePose({ sidebar: ["open", "closed"] });
 *   watch(() => pose.value.sidebar, (s) => s && (open.value = s === "open"), { immediate: true });
 */
export function usePose<const K extends PoseKeys>(keys: K): ComputedRef<Pose<K>> {
  const route = useRoute();
  return computed(() => {
    const pose: Record<string, string> = {};
    for (const [key, accepted] of Object.entries(keys)) {
      const value = route.query[key];
      if (typeof value === "string" && accepted.includes(value)) pose[key] = value;
    }
    return pose as Pose<K>;
  });
}
