import type { Channel, CurrentRelease } from "#shared/utils/releases";

/**
 * The channel's current version, fetched client-side only — `server: false`
 * is load-bearing: a prerendered page's SSR pass runs at build time, before
 * any Worker (and its R2 binding) exists, and a request-time read here
 * would defeat the full-page caching the static HTML is for. See the
 * design's "The download page is static; only the displayed version is
 * live" decision.
 */
export function useCurrentRelease(channel: MaybeRef<Channel>) {
  const url = computed(() => `/api/v1/releases/${toValue(channel)}/current`);
  return useFetch<CurrentRelease | null>(url, {
    key: `current-release-${toValue(channel)}`,
    server: false,
  });
}
