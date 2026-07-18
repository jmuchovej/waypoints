export const REPO = "waypoints-hq/waypoints";

interface GitHubAsset {
  name: string;
  browser_download_url: string;
}

interface GitHubRelease {
  tag_name: string;
  name: string;
  body: string;
  prerelease: boolean;
  draft: boolean;
  published_at: string;
  assets: GitHubAsset[];
}

export interface ReleaseAssets {
  tag: string;
  date: string;
  assets: GitHubAsset[];
}

export async function fetchGitHubReleases(): Promise<GitHubRelease[]> {
  const { github } = useRuntimeConfig();
  const releases = await $fetch<GitHubRelease[]>(`https://api.github.com/repos/${REPO}/releases`, {
    headers: {
      Accept: "application/vnd.github+json",
      ...(github.token && {
        Authorization: `Bearer ${github.token}`,
      }),
    },
  });

  console.log(releases);

  return releases.filter((r) => !r.draft);
}

export type UpdateChannel = "stable" | "preview" | "nightly";

export const UPDATE_CHANNELS: UpdateChannel[] = ["stable", "preview", "nightly"];

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
export function channelForTag(tag: string): UpdateChannel | null {
  if (/-nightly\./.test(tag)) return "nightly";
  if (/-pre\./.test(tag)) return "preview";
  if (/^v\d+\.\d+\.\d+$/.test(tag)) return "stable";
  return null;
}

export function latestForChannel(
  releases: GitHubRelease[],
  channel: UpdateChannel,
): ReleaseAssets | null {
  const match = releases.find((r) => channelForTag(r.tag_name) === channel);

  if (!match) return null;

  return {
    tag: match.tag_name,
    date: match.published_at,
    assets: match.assets,
  };
}

/**
 * Map Tauri target triples to GitHub release asset patterns.
 * `tauri-action` uploads artifacts with platform identifiers in the filename
 * along with `.sig` signature files.
 */
const UPDATER_PLATFORM_MAP: Record<string, { suffix: string; sigSuffix: string }> = {
  "darwin-aarch64": { suffix: ".app.tar.gz", sigSuffix: ".app.tar.gz.sig" },
  "darwin-x86_64": { suffix: ".app.tar.gz", sigSuffix: ".app.tar.gz.sig" },
  "linux-x86_64": { suffix: ".AppImage", sigSuffix: ".AppImage.sig" },
  "windows-x86_64": { suffix: ".nsis.zip", sigSuffix: ".nsis.zip.sig" },
};

/**
 * Map user-facing download targets to GitHub release asset file extensions.
 *
 * URL scheme: /releases/{channel}/{platform}-{arch}
 *   e.g. /releases/stable/macos-arm64
 *
 * Platform values: macos, windows, linux (generic AppImage)
 *   Linux distro-specific packages can be added as needed
 *   (e.g. arch-linux, fedora, etc.)
 *
 * Arch values: arm64, amd64
 */
const DOWNLOAD_TARGET_MAP: Record<string, string[]> = {
  "macos-arm64": [".aarch64.dmg", "_aarch64.dmg"],
  "macos-amd64": [".x64.dmg", "_x64.dmg", "_x86_64.dmg"],
  "linux-amd64": [".AppImage", "_amd64.AppImage", "_x86_64.AppImage"],
  "linux-arm64": ["_aarch64.AppImage"],
  "windows-amd64": [".exe", "_x64-setup.exe", "_x64_en-US.msi"],
  "windows-arm64": ["_arm64-setup.exe"],
  "android-arm64": [".aab", ".apk"],
};

export function resolveDownloadAsset(assets: GitHubAsset[], target: string): GitHubAsset | null {
  const suffixes = DOWNLOAD_TARGET_MAP[target];
  if (!suffixes) return null;

  for (const suffix of suffixes) {
    const asset = assets.find((a) => a.name.endsWith(suffix));
    if (asset) return asset;
  }

  return null;
}

export const SUPPORTED_TARGETS = Object.keys(DOWNLOAD_TARGET_MAP);

interface TauriPlatformEntry {
  url: string;
  signature: string;
}

export type TauriUpdateManifest = {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<string, TauriPlatformEntry>;
} | null;

export async function buildUpdateManifest(channel: UpdateChannel): Promise<TauriUpdateManifest> {
  const releases = await fetchGitHubReleases();
  const latest = latestForChannel(releases, channel);

  if (!latest) return null;

  const platforms: Record<string, TauriPlatformEntry> = {};

  for (const [target, { suffix, sigSuffix }] of Object.entries(UPDATER_PLATFORM_MAP)) {
    const asset = latest.assets.find((a) => a.name.endsWith(suffix));
    const sig = latest.assets.find((a) => a.name.endsWith(sigSuffix));

    if (asset && sig) {
      // Fetch the signature content (it's a small text file)
      const signature = await $fetch<string>(sig.browser_download_url, {
        responseType: "text",
      }).catch(() => "");

      if (signature) {
        platforms[target] = {
          url: asset.browser_download_url,
          signature,
        };
      }
    }
  }

  if (Object.keys(platforms).length === 0) return null;

  return {
    version: latest.tag.replace(/^v/, ""),
    notes: `Release ${latest.tag}`,
    pub_date: latest.date,
    platforms,
  };
}
