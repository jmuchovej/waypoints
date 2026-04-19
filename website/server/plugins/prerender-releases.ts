/**
 * At prerender time, fetch all releases from ungh.cc and register
 * each individual version route so the static generator visits them.
 */
export default defineNitroPlugin((nitroApp) => {
  if (!import.meta.prerender) return;

  nitroApp.hooks.hook("prerender:routes", async (ctx) => {
    try {
      const data = await $fetch<{
        releases: { tag: string; prerelease: boolean; draft: boolean }[];
      }>(`https://ungh.cc/repos/${REPO}/releases`);

      for (const release of data.releases) {
        if (release.draft) continue;
        const channel = release.prerelease ? "preview" : "stable";
        ctx.routes.add(`/releases/${channel}/${release.tag}`);
      }
    } catch (err) {
      console.warn("[prerender-releases] Failed to fetch releases:", err);
    }

    ctx.routes.add("/api/v1/update/stable.json");
    ctx.routes.add("/api/v1/update/preview.json");
  });
});
