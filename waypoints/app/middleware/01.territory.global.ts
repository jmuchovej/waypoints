import { ipc } from "#imports";

export default defineNuxtRouteMiddleware(async (to) => {
  if (import.meta.server) return;
  // A backend is available under Tauri, or (in dev) via the waypoints-serve
  // HTTP backend. In a production browser build there's no backend, so don't
  // gate routing. The try/catch below still no-ops if IPC isn't ready yet.
  if (!("isTauri" in window) && !import.meta.dev) return;

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
