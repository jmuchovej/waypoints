import type { UpdateChannel } from "./releases";

// bucket key  {series}/waypoints-{platform}-{arch}-{version}.{ext}
// public URL  /api/v1/releases/{channel}/{series}/{platform}/{arch}/{download}

export const PLATFORMS = {
  macos: { ext: "dmg", contentType: "application/x-apple-diskimage" },
  "linux-gnu": { ext: "tar.gz", contentType: "application/gzip" },
  windows: { ext: "zip", contentType: "application/zip" },
} as const;

export type Platform = keyof typeof PLATFORMS;
export const PLATFORM_NAMES = Object.keys(PLATFORMS) as Platform[];

export const ARCHES = ["arm64", "amd64"] as const;
export type Arch = (typeof ARCHES)[number];

export function isPlatform(value: string): value is Platform {
  return Object.hasOwn(PLATFORMS, value);
}

export function isArch(value: string): value is Arch {
  return (ARCHES as readonly string[]).includes(value);
}

/** `v0.1.0-nightly.2+2026.10.11` → `v0.1.x`. */
export function seriesFor(version: string): string | null {
  const match = /^v(\d+)\.(\d+)\.\d+/.exec(version);

  return match ? `v${match[1]}.${match[2]}.x` : null;
}

/** Name inside the bucket. */
export function bundleNameFor(platform: Platform, arch: Arch, version: string): string {
  return `waypoints-${platform}-${arch}-${version}.${PLATFORMS[platform].ext}`;
}

export function bundleKeyFor(platform: Platform, arch: Arch, version: string): string | null {
  const series = seriesFor(version);

  return series ? `${series}/${bundleNameFor(platform, arch, version)}` : null;
}

/** Name in the URL, and the only place the version appears — the server reads it back out. */
export function downloadNameFor(platform: Platform, version: string): string {
  return `waypoints-${version}.${PLATFORMS[platform].ext}`;
}

/** Cosmetic name for `Content-Disposition` only; never parsed back, never addresses an object. */
export function savedNameFor(platform: Platform, version: string): string {
  const pretty = version.replace(/-nightly\.\d+\+(\d{4})\.(\d{2})\.(\d{2})$/, "-nightly@$1-$2-$3");

  return `waypoints-${pretty}.${PLATFORMS[platform].ext}`;
}

export function versionFromDownloadName(platform: Platform, name: string): string | null {
  const prefix = "waypoints-";
  const suffix = `.${PLATFORMS[platform].ext}`;

  if (!name.startsWith(prefix) || !name.endsWith(suffix)) return null;

  const version = name.slice(prefix.length, name.length - suffix.length);

  return version.length > 0 ? version : null;
}

export function downloadPathFor(
  channel: UpdateChannel,
  version: string,
  platform: Platform,
  arch: Arch,
): string | null {
  const series = seriesFor(version);

  if (!series) return null;

  // `+` needs no escaping in a path segment; it only means a space in a query string.
  const name = downloadNameFor(platform, version);

  return `/api/v1/releases/${channel}/${series}/${platform}/${arch}/${name}`;
}

export function manifestKeyFor(channel: UpdateChannel): string {
  return `${channel}.json`;
}

export type DesktopTarget = {
  platform: Platform;
  arch: Arch;
  icon: string;
  title: string;
  description: string;
};

export const DESKTOP_TARGETS: DesktopTarget[] = [
  {
    platform: "macos",
    arch: "arm64",
    icon: "i-simple-icons-apple",
    title: "macOS",
    description: "Apple Silicon",
  },
  {
    platform: "macos",
    arch: "amd64",
    icon: "i-simple-icons-apple",
    title: "macOS",
    description: "Intel",
  },
  {
    platform: "linux-gnu",
    arch: "amd64",
    icon: "i-simple-icons-linux",
    title: "Linux",
    description: "x86_64 (glibc)",
  },
  {
    platform: "linux-gnu",
    arch: "arm64",
    icon: "i-simple-icons-linux",
    title: "Linux",
    description: "ARM64 (glibc)",
  },
  {
    platform: "windows",
    arch: "amd64",
    icon: "i-simple-icons-windows",
    title: "Windows",
    description: "64-bit",
  },
];

export type StoreLink = {
  icon: string;
  title: string;
  description: string;
  to: string;
};

// Per `fastlane/Fastfile`, stable on Play is the only publicly reachable listing: iOS is
// never submitted to the App Store (`skip_submission`), and preview/nightly are internal-track.
export const STORE_LINKS: Record<UpdateChannel, StoreLink[]> = {
  stable: [
    {
      icon: "i-simple-icons-googleplay",
      title: "Android",
      description: "Google Play",
      to: "https://play.google.com/store/apps/details?id=so.waypoints",
    },
  ],
  preview: [],
  nightly: [],
};
