import { channelOf } from "#shared/utils/releases";

import { REPO } from "../utils/notes";

/**
 * At prerender time, list releases from the forge (not the store — the R2
 * binding only exists inside a deployed Worker, never during the build) and
 * register each individual version route so the static generator visits it.
 *
 * The update-manifest routes are deliberately **not** added here any more:
 * they read the live store through `buildUpdateManifest`, so baking one
 * answer into the build would freeze the updater on whatever version
 * happened to be current at build time.
 */
export default defineNitroPlugin((nitroApp) => {
  if (!import.meta.prerender) return;

  // `prerender:routes` is a Nitro build hook (payload is a `Set<string>` of
  // routes); it isn't part of NitroRuntimeHooks, so the hooks object is cast.
  (nitroApp.hooks as any).hook("prerender:routes", async (routes: Set<string>) => {
    try {
      const { github } = useRuntimeConfig();
      const releases = await $fetch<{ tag_name: string; prerelease: boolean; draft: boolean }[]>(
        `https://api.github.com/repos/${REPO}/releases`,
        {
          headers: {
            Accept: "application/vnd.github+json",
            ...(github.token && { Authorization: `Bearer ${github.token}` }),
          },
        },
      );

      for (const release of releases) {
        if (release.draft) continue;
        const channel = channelOf(release.tag_name);
        // Nightlies are an ephemeral dev channel — don't prerender a marketing
        // notes page per build. Only stable/preview get per-release pages.
        if (channel !== "stable" && channel !== "preview") continue;
        routes.add(`/releases/${channel}/${release.tag_name}`);
      }
    } catch (err) {
      console.warn("[prerender-releases] Failed to list releases:", err);
    }
  });
});
