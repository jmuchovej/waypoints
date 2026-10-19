/**
 * Release notes, read server-side from the forge rather than the store —
 * see the design's "Notes stay in the forge; enumeration moves to the
 * store" decision. Replaces the old `ungh.cc` fetch, which is unauthenticated
 * and cannot read this private repository.
 */

export const REPO = "waypoints-hq/waypoints";

interface GitHubRelease {
  name: string;
  body: string;
  published_at: string;
}

export interface ReleaseNotes {
  title: string;
  date: string;
  markdown: string;
}

/**
 * One release's notes by tag, or `null` when the forge has nothing under
 * that tag — a deleted draft, a withdrawn nightly, or the forge being
 * unreachable. A missing or unreadable release still lists with its version
 * and downloads; see the design's risk on this and task 4.5.
 */
export async function fetchReleaseNotes(tag: string): Promise<ReleaseNotes | null> {
  const { github } = useRuntimeConfig();
  try {
    const release = await $fetch<GitHubRelease>(
      `https://api.github.com/repos/${REPO}/releases/tags/${tag}`,
      {
        headers: {
          Accept: "application/vnd.github+json",
          ...(github.token && { Authorization: `Bearer ${github.token}` }),
        },
      },
    );
    return { title: release.name || tag, date: release.published_at, markdown: release.body };
  } catch {
    return null;
  }
}
