export const UPDATE_CHANNELS = ["stable", "preview", "nightly"] as const;
export type UpdateChannel = (typeof UPDATE_CHANNELS)[number];

// Must match what `nightly_plan` mints: `v0.1.0-nightly.2+2026.10.11`.
export const NIGHTLY_RE = /-nightly\.\d+\+\d{4}\.\d{2}\.\d{2}$/;
export const PREVIEW_RE = /-pre\.\d+$/;
export const STABLE_RE = /^v\d+\.\d+\.\d+$/;

/** Classify by shape alone, for versions with no GitHub release to consult. Most-specific first. */
export function channelForVersion(tag: string): UpdateChannel | null {
  if (NIGHTLY_RE.test(tag)) return "nightly";
  if (PREVIEW_RE.test(tag)) return "preview";
  if (STABLE_RE.test(tag)) return "stable";

  return null;
}

// Shape decides the channel; `prerelease` only has to agree. The flag alone cannot separate
// nightly from preview, since both are GitHub prereleases.
export function channelForRelease(tag: string, is_prerelease?: boolean): UpdateChannel | null {
  const channel = channelForVersion(tag);

  if (!channel) return null;

  return (channel === "stable") === Boolean(is_prerelease) ? null : channel;
}
