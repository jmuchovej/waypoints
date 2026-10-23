import { isChannel, type CurrentRelease } from "#shared/utils/releases";

import { fetchReleaseNotes } from "../../../../../utils/notes";
import { readManifest } from "../../../../../utils/store";

/**
 * The channel's current version and its display metadata — the light read
 * the download page's heading needs, distinct from `useReleases`' full
 * enumeration. Meant to be fetched client-side only: see the design's "The
 * download page is static; only the displayed version is live" decision.
 */
export default defineEventHandler(async (event): Promise<CurrentRelease | null> => {
  const channel = getRouterParam(event, "channel");
  if (!channel || !isChannel(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }

  const manifest = await readManifest(event, channel);
  if (!manifest) return null;

  const notes = await fetchReleaseNotes(manifest.version);
  return {
    tag: manifest.version,
    title: notes?.title ?? manifest.version,
    date: notes?.date ?? null,
  };
});
