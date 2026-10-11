import { UPDATE_CHANNELS, type UpdateChannel, channelForRelease } from "#shared/utils/releases";

/**
 * At prerender time, fetch all releases from ungh.cc and register
 * each individual version route so the static generator visits them.
 */
export default defineNitroPlugin((nitroApp) => {
  if (!import.meta.prerender) return;

  // `prerender:routes` is a Nitro build hook (payload is a `Set<string>` of
  // routes); it isn't part of NitroRuntimeHooks, so the hooks object is cast.
  (nitroApp.hooks as any).hook("prerender:routes", async (routes: Set<string>) => {
    try {
      const data = await $fetch<{
        releases: { tag: string; prerelease: boolean; draft: boolean }[];
      }>(`https://ungh.cc/repos/${REPO}/releases`);

      for (const release of data.releases) {
        if (release.draft) continue;
        const channel = channelForRelease(release.tag, release.prerelease);
        // Nightlies are an ephemeral dev channel — don't prerender a marketing
        // notes page per build. Only stable/preview get per-release pages.
        if (!channel || !UPDATE_CHANNELS.includes(channel)) continue;
        routes.add(`/releases/${channel}/${release.tag}`);
      }
    } catch (err) {
      console.warn("[prerender-releases] Failed to fetch releases:", err);
    }

    // Updater manifests are not prerendered: the R2 binding they read does not exist at build
    // time, and baking them would freeze a version until the next deploy.
  });
});
