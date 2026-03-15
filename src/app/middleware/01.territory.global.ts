import { commands } from "~/bindings.g";

export default defineNuxtRouteMiddleware(async (to) => {
  if (import.meta.server) return;
  if (import.meta.dev) return;

  try {
    const result = await commands.territoryList();
    if (result.status !== "ok") return;
    const hasTerritory = result.data.length > 0;

    if (!hasTerritory && to.path !== "/onboarding") {
      return navigateTo("/onboarding");
    }

    if (hasTerritory && to.path === "/onboarding") {
      return navigateTo("/");
    }
  } catch {
    // Tauri IPC not ready (HMR, web dev) — don't gate
    return;
  }
});
