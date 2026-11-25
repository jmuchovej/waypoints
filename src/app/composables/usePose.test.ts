// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { mountSuspended } from "@nuxt/test-utils/runtime";
import { describe, expect, it } from "vitest";
import { defineComponent, h } from "vue";

import { usePose } from "./usePose";

const KEYS = { sidebar: ["open", "closed"] } as const;

async function poseAt(route: string) {
  let pose!: ReturnType<typeof usePose<typeof KEYS>>;
  await mountSuspended(
    defineComponent({
      setup() {
        pose = usePose(KEYS);
        return () => h("div");
      },
    }),
    { route },
  );
  return pose.value;
}

describe("usePose", () => {
  it("reads a declared key", async () => {
    expect(await poseAt("/library/recents?sidebar=open")).toEqual({ sidebar: "open" });
  });

  it("ignores keys the page does not declare", async () => {
    expect(await poseAt("/library/recents?sidebar=closed&delete=all")).toEqual({
      sidebar: "closed",
    });
  });

  it("ignores values the page does not accept", async () => {
    expect(await poseAt("/library/recents?sidebar=wide")).toEqual({});
  });

  it("ignores a repeated key", async () => {
    expect(await poseAt("/library/recents?sidebar=open&sidebar=closed")).toEqual({});
  });

  it("never writes back to the route", async () => {
    const route = "/library/recents?sidebar=open&other=1";
    await poseAt(route);
    expect(useRouter().currentRoute.value.fullPath).toBe(route);
  });
});
