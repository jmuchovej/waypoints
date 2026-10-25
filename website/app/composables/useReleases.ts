import type { Channel } from "#shared/utils/releases";

export interface Release {
  tag: string;
  title: string;
  date: string | null;
  prerelease: boolean;
  /** `null` when the forge's notes for this tag couldn't be read — the
   * release still lists with its version, just without a changelog body.
   * See the design's "Notes stay in the forge" decision and task 4.5. */
  markdown: string | null;
}

/**
 * Every version the store holds for `channel`, newest first, with notes
 * read server-side from the forge. Replaces the old `ungh.cc` fetch, which
 * is unauthenticated and cannot read this private repository.
 */
export function useReleases(channel: MaybeRef<Channel>) {
  const url = computed(() => `/api/v1/releases/${toValue(channel)}`);

  return useFetch(url, {
    key: `releases-${toValue(channel)}`,
    transform: (data: { releases: Release[] }) => data.releases,
  });
}
