import ipc from "~/ipc";

export default defineNuxtRouteMiddleware(async (to) => {
  if (import.meta.server) return;
  if (import.meta.dev) return;

  try {
    const territories = await ipc.territory.list();
    const hasTerritory = territories.length > 0;

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
