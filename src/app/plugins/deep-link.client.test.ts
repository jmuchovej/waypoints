// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { mockNuxtImport } from "@nuxt/test-utils/runtime";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import plugin from "./deep-link.client";

// ── Harness ─────────────────────────────────────────────────────────────
//
// `ipc` records every call. Only the reads the handler and the routing
// middleware need answer; anything else is a call a link should never cause.

const { calls, incoming, device } = vi.hoisted(() => ({
  calls: [] as string[],
  incoming: { urls: [] as string[] },
  device: { onboarded: true },
}));

const READS: Record<string, (...args: unknown[]) => unknown> = {
  "sync.status": () => ({ hasIdentity: device.onboarded }),
  "territory.list": () => (device.onboarded ? [{ id: "demo" }] : []),
  "sync.classifySetupCode": (input) =>
    String(input).includes("#") ? { kind: "pairing", value: "{}" } : { kind: "unknown" },
};

mockNuxtImport("ipc", () => {
  const domain = (name: string) =>
    new Proxy(
      {},
      {
        get:
          (_, fn: string) =>
          async (...args: unknown[]) => {
            calls.push(`${name}.${fn}`);
            return READS[`${name}.${fn}`]?.(...args);
          },
      },
    );
  return new Proxy({}, { get: (_, name: string) => domain(name) });
});

vi.mock("@tauri-apps/plugin-deep-link", () => ({
  getCurrent: async () => incoming.urls,
  onOpenUrl: async () => () => {},
}));

const route = () => useRouter().currentRoute.value;

/** Deliver `urls` as the links that launched the app. */
async function open(...urls: string[]) {
  incoming.urls = urls;
  const nuxtApp = useNuxtApp();
  await nuxtApp.runWithContext(() => (plugin as (app: typeof nuxtApp) => unknown)(nuxtApp));
  // The handler holds links until the app's first render. Here the app
  // rendered before the plugin was installed, so signal that render again.
  await nuxtApp.callHook("app:suspense:resolve");
}

/** A navigation loads the target page's chunk first, so give it time. */
async function landsOn(path: string) {
  await vi.waitFor(() => expect(route().path).toBe(path), { timeout: 15_000 });
}

/** For the cases where nothing should happen: let the detached handler run. */
async function settle() {
  await new Promise((r) => setTimeout(r, 200));
}

function mutations() {
  return calls.filter((call) => !(call in READS));
}

beforeEach(async () => {
  Object.assign(window, { isTauri: true });
  device.onboarded = true;
  await useRouter().replace("/settings");
  calls.length = 0;
  useState("sync-pending-setup-code").value = null;
});

afterEach(() => {
  delete (window as { isTauri?: boolean }).isTauri;
});

// ── Route links ─────────────────────────────────────────────────────────

describe("a route link", () => {
  it("lands on the page it names, query and hash intact", async () => {
    await open("waypoints://library/recents?sidebar=open#top");
    await landsOn("/library/recents");
    expect(route().fullPath).toBe("/library/recents?sidebar=open#top");
  });

  it("lands on a parameterised route", async () => {
    await open("waypoints-nightly://marks/0084d8df-ded3-4f52-a01b-5958592e95c1");
    await landsOn("/marks/0084d8df-ded3-4f52-a01b-5958592e95c1");
  });

  it("is ignored when no page serves its path", async () => {
    await open("waypoints://nowhere/at-all");
    await settle();
    expect(route().fullPath).toBe("/settings");
  });

  it("is ignored under a scheme that is not a flavor's", async () => {
    await open("elsewhere://library/recents");
    await settle();
    expect(route().fullPath).toBe("/settings");
  });

  it("navigates and never acts", async () => {
    await open("waypoints-preview://library/recents?sidebar=open&delete=everything");
    await landsOn("/library/recents");
    expect(mutations()).toEqual([]);
    expect(calls).not.toContain("sync.classifySetupCode");
  });
});

// ── Setup-code links (the pairing path) ─────────────────────────────────

describe("a link carrying a setup code", () => {
  // Pairing is for a device not yet set up; the routing gate sends an
  // onboarded one away from /onboarding.
  beforeEach(() => {
    device.onboarded = false;
  });

  it.each(["waypoints", "waypoints-preview", "waypoints-nightly"])(
    "prefills onboarding from %s:// and submits nothing",
    async (scheme) => {
      const url = `${scheme}://pair#e30`;
      await open(url);
      await landsOn("/onboarding");
      expect(useState("sync-pending-setup-code").value).toBe(url);
      expect(mutations()).toEqual([]);
    },
  );

  it("arrives the same way as a universal link", async () => {
    const url = "https://waypoints.so/pair#e30";
    await open(url);
    await landsOn("/onboarding");
    expect(useState("sync-pending-setup-code").value).toBe(url);
    expect(mutations()).toEqual([]);
  });

  it("is ignored when the code does not classify", async () => {
    await open("waypoints://pair");
    await settle();
    expect(route().fullPath).toBe("/settings");
    expect(useState("sync-pending-setup-code").value).toBeNull();
  });
});
