export const UPDATE_CHANNELS = ["stable", "preview", "nightly"] as const;
export type UpdateChannel = (typeof UPDATE_CHANNELS)[number];

export const NIGHTLY_RE = /-nightly\.\d{4}\d{2}\d{2}$/;
export const PREVIEW_RE = /-pre\.\d+$/;
export const STABLE_RE = /^v\d+\.\d+\.\d+$/;

/**
 * Classify a release tag into an update channel by its shape.
 *
 * Preview and nightly are both GitHub *prereleases*, so the `prerelease`
 * boolean alone can't tell them apart — a nightly would otherwise be served as
 * the latest "preview". Match on the tag suffix instead, most-specific first:
 *   nightly  vX.Y.Z-nightly.<date>
 *   preview  vX.Y.Z-pre.<n>
 *   stable   vX.Y.Z
 */
export function channelForRelease(tag: string, is_prerelease?: boolean): UpdateChannel | null {
  if (is_prerelease && NIGHTLY_RE.test(tag)) return "nightly";
  if (is_prerelease && PREVIEW_RE.test(tag)) return "preview";
  if (!is_prerelease && STABLE_RE.test(tag)) return "stable";

  return null;
}
