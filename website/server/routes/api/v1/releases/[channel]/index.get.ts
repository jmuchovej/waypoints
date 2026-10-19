import { isChannel, compareVersionsDescending } from "#shared/utils/releases";

import { fetchReleaseNotes } from "../../../../../utils/notes";
import { listVersions } from "../../../../../utils/store";

export interface Release {
  tag: string;
  title: string;
  date: string | null;
  prerelease: boolean;
  markdown: string | null;
}

/**
 * Every version the store holds for `channel`, newest first, with notes
 * read server-side from the forge. A version whose notes can't be read
 * still lists — with its tag and `date`/`markdown` left `null` — rather
 * than being dropped, since the store already confirms it was published;
 * see task 4.5.
 */
export default defineEventHandler(async (event): Promise<{ releases: Release[] }> => {
  const channel = getRouterParam(event, "channel");
  if (!channel || !isChannel(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }

  const versions = (await listVersions(event, channel)).sort(compareVersionsDescending);

  const releases = await Promise.all(
    versions.map(async (tag): Promise<Release> => {
      const notes = await fetchReleaseNotes(tag);
      return {
        tag,
        title: notes?.title ?? tag,
        date: notes?.date ?? null,
        prerelease: channel !== "stable",
        markdown: notes?.markdown ?? null,
      };
    }),
  );

  return { releases };
});
