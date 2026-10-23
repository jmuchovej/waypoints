/**
 * Channel, target, and object-key modeling for the `releases` R2 store.
 *
 * Mirrors `tooling/xtask/src/tasks/workflows/_types.rs` and
 * `tooling/xtask/src/tasks/release_store.rs` by convention — nothing here is
 * generated, so a change to the Rust shape has to be carried over by hand.
 */

export type Channel = "nightly" | "preview" | "stable";
export const CHANNELS: Channel[] = ["nightly", "preview", "stable"];

export function isChannel(value: string): value is Channel {
  return (CHANNELS as string[]).includes(value);
}

export interface ReleaseTarget {
  os: string;
  arch: string;
  extensions: string[];
}

/** Every target a channel's manifest can name — see `Channel::required_targets`. */
export const REQUIRED_TARGETS: ReleaseTarget[] = [
  { os: "macos", arch: "arm64", extensions: ["dmg"] },
  { os: "windows", arch: "amd64", extensions: ["exe"] },
  { os: "linux", arch: "amd64", extensions: ["AppImage", "deb", "rpm"] },
  { os: "ios", arch: "arm64", extensions: ["ipa"] },
  { os: "android", arch: "arm64", extensions: ["apk", "aab"] },
];

/**
 * The subset with an actual download path. Mobile ships through its own app
 * store and is archived in the bucket without ever being linked here — see
 * the design's "Published formats are the bundler's installers" decision.
 */
export const DOWNLOADABLE_TARGETS = REQUIRED_TARGETS.filter((target) =>
  ["macos", "linux", "windows"].includes(target.os),
);

/** The user-facing `{os}-{arch}` target spelling the download routes take. */
export function targetSlug(target: Pick<ReleaseTarget, "os" | "arch">): string {
  return `${target.os}-${target.arch}`;
}

export interface UpdaterTarget {
  os: string;
  arch: string;
  /** The extension of the file the updater downloads and installs. */
  payload: string;
  /** The extension of that file's updater signature. */
  signature: string;
}

/**
 * Tauri's own updater target spellings, each mapped to our `{os, arch}` and
 * the payload it installs — mirrors `Platform::updater_extensions`. Linux
 * and Windows sign the installer in place, so the payload is the file
 * already published; macOS's updater cannot consume a `.dmg`, so its payload
 * is the separate `.app.tar.gz`.
 *
 * An app installed from a deb or rpm asks for `{os}-{arch}-{bundle_type}`
 * before the bare `{os}-{arch}`, and refuses a payload of any other format,
 * so each Linux installer has its own entry. The bare `linux-x86_64` is the
 * AppImage.
 */
export const TAURI_UPDATER_TARGETS: Record<string, UpdaterTarget> = {
  "darwin-aarch64": {
    os: "macos",
    arch: "arm64",
    payload: "app.tar.gz",
    signature: "app.tar.gz.sig",
  },
  "linux-x86_64": { os: "linux", arch: "amd64", payload: "AppImage", signature: "AppImage.sig" },
  "linux-x86_64-deb": { os: "linux", arch: "amd64", payload: "deb", signature: "deb.sig" },
  "linux-x86_64-rpm": { os: "linux", arch: "amd64", payload: "rpm", signature: "rpm.sig" },
  "windows-x86_64": { os: "windows", arch: "amd64", payload: "exe", signature: "exe.sig" },
};

/**
 * Classifies a version string into the channel its tag grammar names —
 * mirrors `release_plan::channel_of`. Preview and nightly are both
 * pre-releases, so the suffix has to be matched, most specific first.
 */
export function channelOf(version: string): Channel {
  if (/-nightly\./.test(version)) return "nightly";
  if (/-pre\./.test(version)) return "preview";
  return "stable";
}

/** The series a version belongs to, e.g. `v0.1.1` → `v0.1.x`. */
export function seriesOf(version: string): string {
  const match = /^v?(\d+)\.(\d+)\./.exec(version);
  if (!match) throw new Error(`not a version: ${version}`);
  return `v${match[1]}.${match[2]}.x`;
}

export function manifestKey(channel: Channel): string {
  return `${channel}.json`;
}

/**
 * The store key for one artifact, in the same
 * `{channel}/{series}/{os}/{arch}/waypoints-{version}.{ext}` shape the
 * uploader writes. Fails closed on a channel/version mismatch rather than
 * filing — or reading — under the wrong prefix, the same rule the uploader
 * and the download routes both follow; see the design's "Channel leads the
 * key" decision.
 */
export function objectKey(
  channel: Channel,
  version: string,
  os: string,
  arch: string,
  extension: string,
): string {
  const actual = channelOf(version);
  if (actual !== channel) {
    throw new Error(`version ${version} names channel ${actual}, not ${channel}`);
  }
  return `${channel}/${seriesOf(version)}/${os}/${arch}/waypoints-${version}.${extension}`;
}

/** Every extension an object key can end in, longest first so a suffix
 * match never stops at a shorter extension nested inside a longer one
 * (`app.tar.gz.sig` before `sig`, `AppImage.sig` before `AppImage`). */
const KNOWN_EXTENSIONS = [
  ...new Set([
    ...REQUIRED_TARGETS.flatMap((target) => target.extensions),
    ...Object.values(TAURI_UPDATER_TARGETS).flatMap((target) => [target.payload, target.signature]),
  ]),
].sort((a, b) => b.length - a.length);

/**
 * Recovers the version named by one object key's filename — the inverse of
 * `objectKey`, used to enumerate what the bucket holds rather than to
 * derive a key the uploader already wrote. `null` for a key this scheme
 * did not produce.
 */
export function versionFromKey(key: string): string | null {
  const filename = key.slice(key.lastIndexOf("/") + 1);
  const prefix = "waypoints-";
  if (!filename.startsWith(prefix)) return null;
  const rest = filename.slice(prefix.length);
  for (const extension of KNOWN_EXTENSIONS) {
    if (rest.endsWith(`.${extension}`)) {
      return rest.slice(0, rest.length - extension.length - 1);
    }
  }
  return null;
}

export interface ReleaseManifest {
  version: string;
  series: string;
  targets: ReleaseTarget[];
}

/** The channel's current version and its display metadata — the light read
 * the download page's heading needs, distinct from a full enumeration. */
export interface CurrentRelease {
  tag: string;
  title: string;
  date: string | null;
}

/**
 * The versionless alias URL a download link points at: a mutable pointer
 * in front of the immutable, fully-versioned URL. See the design's `latest`
 * decision — the saved filename never names the alias, only the resolved
 * version it redirects to.
 */
export function latestUrl(channel: Channel, os: string, arch: string, extension: string): string {
  return `/api/v1/releases/${channel}/latest/${os}/${arch}/waypoints-latest.${extension}`;
}

/** The fully-versioned URL a `latest` alias redirects to — the store key
 * itself, mounted under the download route's prefix. */
export function versionedUrl(
  channel: Channel,
  version: string,
  os: string,
  arch: string,
  extension: string,
): string {
  return `/api/v1/releases/${objectKey(channel, version, os, arch, extension)}`;
}

/**
 * Orders versions newest-first. Semver precedence in full is more than
 * this needs: numeric major/minor/patch decide almost every real
 * comparison, and the rest (prerelease/build-metadata) is compared as
 * plain text, which is wrong only for cases this project's tag grammar
 * doesn't produce (e.g. `-pre.9` sorting before `-pre.10`).
 */
export function compareVersionsDescending(a: string, b: string): number {
  const parse = (version: string) => {
    const match = /^v?(\d+)\.(\d+)\.(\d+)(.*)$/.exec(version);
    if (!match) return null;
    return {
      major: Number(match[1]),
      minor: Number(match[2]),
      patch: Number(match[3]),
      rest: match[4] ?? "",
    };
  };
  const left = parse(a);
  const right = parse(b);
  if (!left || !right) return b.localeCompare(a);
  if (left.major !== right.major) return right.major - left.major;
  if (left.minor !== right.minor) return right.minor - left.minor;
  if (left.patch !== right.patch) return right.patch - left.patch;
  return right.rest.localeCompare(left.rest);
}
