// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { mockNuxtImport, mountSuspended } from "@nuxt/test-utils/runtime";
import { flushPromises } from "@vue/test-utils";
import type { SelfUpdatePolicy, SelfUpdateStatus } from "@waypoints/cartograph";
import { beforeEach, describe, expect, it, vi } from "vitest";

import Updates from "./updates.vue";

const { selfUpdateStatus, selfUpdateSet } = vi.hoisted(() => ({
  selfUpdateStatus: vi.fn(),
  selfUpdateSet: vi.fn(),
}));

mockNuxtImport("ipc", () => ({ app: { selfUpdateStatus, selfUpdateSet } }));

const ORDER: SelfUpdatePolicy[] = ["off", "notify", "on"];

function status(setting: SelfUpdatePolicy, cap: SelfUpdatePolicy): SelfUpdateStatus {
  const effective = ORDER.indexOf(setting) <= ORDER.indexOf(cap) ? setting : cap;
  return { setting, cap, effective };
}

async function mount(initial: SelfUpdateStatus) {
  selfUpdateStatus.mockResolvedValue(initial);
  const wrapper = await mountSuspended(Updates);
  await flushPromises();
  return wrapper;
}

type Wrapper = Awaited<ReturnType<typeof mount>>;

const radio = (wrapper: Wrapper, value: SelfUpdatePolicy) =>
  wrapper.find(`button[role="radio"][value="${value}"]`);

const isDisabled = (wrapper: Wrapper, value: SelfUpdatePolicy) =>
  radio(wrapper, value).attributes("disabled") !== undefined ||
  radio(wrapper, value).attributes("data-disabled") !== undefined;

describe("settings/app/updates", () => {
  beforeEach(() => {
    selfUpdateStatus.mockReset();
    selfUpdateSet.mockReset();
  });

  it("offers every value and no notice when the launch set no cap", async () => {
    const wrapper = await mount(status("on", "on"));

    for (const value of ORDER) {
      expect(radio(wrapper, value).exists()).toBe(true);
      expect(isDisabled(wrapper, value)).toBe(false);
    }
    expect(wrapper.find('[data-testid="managed-notice"]').exists()).toBe(false);
  });

  it("disables `on` and shows the notice under a cap, leaving the rest selectable", async () => {
    const wrapper = await mount(status("on", "notify"));

    expect(isDisabled(wrapper, "on")).toBe(true);
    expect(isDisabled(wrapper, "notify")).toBe(false);
    expect(isDisabled(wrapper, "off")).toBe(false);

    const notice = wrapper.find('[data-testid="managed-notice"]');
    expect(notice.exists()).toBe(true);
    expect(notice.text()).toContain(
      "Self-updating is disabled because this copy of Waypoints is managed by a package manager, which delivers updates itself.",
    );
    expect(notice.text()).not.toMatch(/nix|arch|homebrew|brew|pacman/i);
    expect(notice.find("a").attributes("href")).toBe(
      "https://waypoints.so/docs/installation/managed-installs",
    );
  });

  it("writes a chosen value through the backend and shows what came back", async () => {
    const wrapper = await mount(status("on", "notify"));
    selfUpdateSet.mockResolvedValue(status("off", "notify"));

    await radio(wrapper, "off").trigger("click");
    await flushPromises();

    expect(selfUpdateSet).toHaveBeenCalledWith("off");
    expect(radio(wrapper, "off").attributes("aria-checked")).toBe("true");
  });
});
