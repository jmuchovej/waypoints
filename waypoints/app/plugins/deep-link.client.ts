import { ipc } from "#imports";

/**
 * Route incoming deep links (waypoints:// or associated waypoints.so
 * links) into the setup flow. The link's payload is classified but never
 * auto-submitted — it lands prefilled in the onboarding input so the user
 * confirms the join; adopting an identity is not something a URL should do
 * on its own.
 */
export default defineNuxtPlugin(() => {
  if (!("isTauri" in window)) return;

  const pending = useState<string | null>("sync-pending-setup-code", () => null);

  async function handle(urls: string[]) {
    for (const url of urls) {
      if ((await ipc.sync.classifySetupCode(url)).kind === "unknown") continue;
      pending.value = url;
      await navigateTo("/onboarding");
      return;
    }
  }

  (async () => {
    const { getCurrent, onOpenUrl } = await import("@tauri-apps/plugin-deep-link");
    // A cold start launched by a link delivers it via getCurrent; a running
    // instance gets onOpenUrl events (including ones forwarded by
    // single-instance).
    const initial = await getCurrent().catch(() => null);
    if (initial?.length) await handle(initial);
    await onOpenUrl((urls) => void handle(urls));
  })().catch((e) => console.error("deep-link setup failed", e));
});
