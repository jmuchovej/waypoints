// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { ipc } from "#imports";

/**
 * Route incoming deep links. A link is one of two kinds, decided before
 * anything else looks at it:
 *
 * - **Setup code** — `<scheme>://pair#…` or `https://waypoints.so/pair#…`.
 *   Classified in Rust and left prefilled in the onboarding input.
 * - **Route** — `<scheme>://library/recents?sidebar=open`. Host and path name a
 *   page under `src/app/pages/`; path, query and hash reach the router
 *   unchanged, and the page reads any presentation state through `usePose`.
 *   A path no page serves is ignored.
 *
 * `<scheme>` is the flavor's: `waypoints` (stable and development),
 * `waypoints-preview`, `waypoints-nightly`. Anything else is ignored.
 *
 * A link navigates; it never acts. It may take the app to a page and pose
 * it, but never perform what that page offers — a setup code is never
 * submitted (adopting an identity is not something a URL should do on its
 * own), and no route or pose creates, edits or deletes anything. Every kind
 * added here keeps to that.
 */

const APP_SCHEME = /^waypoints(-[a-z]+)?:$/;

type Link = { kind: "setup" } | { kind: "route"; to: string } | { kind: "unknown" };

function kindOf(raw: string): Link {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return { kind: "unknown" };
  }
  if (url.protocol === "https:") {
    return url.host === "waypoints.so" && url.pathname === "/pair"
      ? { kind: "setup" }
      : { kind: "unknown" };
  }
  if (!APP_SCHEME.test(url.protocol)) return { kind: "unknown" };
  // `waypoints://library/recents` parses with host "library", path "/recents".
  const path = `/${url.host}${url.pathname}`.replace(/\/+$/, "") || "/";
  if (path === "/pair") return { kind: "setup" };
  return { kind: "route", to: `${path}${url.search}${url.hash}` };
}

export default defineNuxtPlugin((nuxtApp) => {
  if (!("isTauri" in window)) return;

  const router = useRouter();
  const pending = useState<string | null>("sync-pending-setup-code", () => null);

  async function handle(urls: string[]) {
    for (const url of urls) {
      const link = kindOf(url);
      if (link.kind === "setup") {
        if ((await ipc.sync.classifySetupCode(url)).kind === "unknown") continue;
        pending.value = url;
        await navigateTo("/onboarding");
        return;
      }
      if (link.kind === "route") {
        const target = router.resolve(link.to);
        if (!target.matched.length) continue;
        await navigateTo(target.fullPath);
        return;
      }
    }
  }

  // A link is handled only once the app has rendered its first page. Until
  // then the landing navigation, and the page it suspends on, would overtake
  // the link that launched the app: `router.isReady()` alone resolves before
  // that page has finished rendering.
  const rendered = new Promise<void>((resolve) =>
    nuxtApp.hooks.hookOnce("app:suspense:resolve", () => resolve()),
  );
  const ready = Promise.all([router.isReady(), rendered]);
  const handleWhenReady = async (urls: string[]) => {
    await ready;
    await handle(urls);
  };

  (async () => {
    const { getCurrent, onOpenUrl } = await import("@tauri-apps/plugin-deep-link");
    // A cold start launched by a link delivers it via getCurrent; a running
    // instance gets onOpenUrl events (including ones forwarded by
    // single-instance).
    await onOpenUrl((urls) => void handleWhenReady(urls));
    const initial = await getCurrent().catch(() => null);
    if (initial?.length) await handleWhenReady(initial);
  })().catch((e) => console.error("deep-link setup failed", e));
});
