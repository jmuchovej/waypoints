import type { H3Event } from "h3";

import {
  TAURI_UPDATER_TARGETS,
  UPDATER_PAYLOAD_EXT,
  UPDATER_SIG_EXT,
  objectKey,
  type Channel,
} from "#shared/utils/releases";

import { fetchReleaseNotes } from "./notes";
import { readManifest, readText } from "./store";

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

/**
 * Builds the Tauri updater's manifest for `channel` entirely from the
 * store: the current version comes from the channel's manifest, and each
 * platform's payload is served by this site's own streaming route rather
 * than a GitHub release asset — release builds no longer attach any (see
 * the design's "Published formats are the bundler's installers" decision
 * and the proposal's note on dropping GitHub release assets).
 */
export async function buildUpdateManifest(
  event: H3Event,
  channel: Channel,
): Promise<TauriUpdateManifest> {
  const manifest = await readManifest(event, channel);
  if (!manifest) return null;

  const platforms: Record<string, TauriPlatformEntry> = {};

  for (const [tauriTarget, { os, arch }] of Object.entries(TAURI_UPDATER_TARGETS)) {
    const sigExt = UPDATER_SIG_EXT[os];
    if (!sigExt) continue;

    const sigKey = objectKey(channel, manifest.version, os, arch, sigExt);
    const signature = await readText(event, sigKey);
    if (!signature) continue;

    platforms[tauriTarget] = {
      url: `/api/v1/releases/${channel}/update/${tauriTarget}`,
      signature,
    };
  }

  if (Object.keys(platforms).length === 0) return null;

  // The store names what exists but not when it was published; the forge
  // does — see the design's "Notes stay in the forge" decision. Falls back
  // to "now" only if the forge has nothing under this tag, same as a
  // missing-notes release still lists.
  const notes = await fetchReleaseNotes(manifest.version);

  return {
    version: manifest.version.replace(/^v/, ""),
    notes: notes?.markdown || `Release ${manifest.version}`,
    pub_date: notes?.date ?? new Date().toISOString(),
    platforms,
  };
}

/** The store key for `tauriTarget`'s updater payload under `channel`'s current version. */
export async function updaterPayloadKey(
  event: H3Event,
  channel: Channel,
  tauriTarget: string,
): Promise<string | null> {
  const target = TAURI_UPDATER_TARGETS[tauriTarget];
  const payloadExt = target && UPDATER_PAYLOAD_EXT[target.os];
  if (!target || !payloadExt) return null;

  const manifest = await readManifest(event, channel);
  if (!manifest) return null;

  return objectKey(channel, manifest.version, target.os, target.arch, payloadExt);
}
