import { DOWNLOADABLE_TARGETS, isChannel, targetSlug, versionedUrl } from "#shared/utils/releases";

import { readManifest } from "../../../../../../../../utils/store";

/**
 * A mutable pointer in front of the immutable, fully-versioned URL — never
 * serves bytes itself, only resolves the channel's current manifest and
 * redirects. Not cached as immutable: unlike the URL it points at, this
 * response changes on every publish. See the design's "`latest` is a
 * redirect to the versioned URL" decision.
 *
 * The incoming `file` segment (`waypoints-latest.{ext}`) is decorative —
 * the real extension comes from the manifest, since each downloadable
 * target has exactly one. This route never reads it back.
 */
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel");
  const os = getRouterParam(event, "os");
  const arch = getRouterParam(event, "arch");

  if (!channel || !isChannel(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }

  const downloadable = DOWNLOADABLE_TARGETS.find(
    (target) => target.os === os && target.arch === arch,
  );
  if (!downloadable) {
    throw createError({ statusCode: 404, statusMessage: `Unknown target: ${os}-${arch}` });
  }

  const manifest = await readManifest(event, channel);
  const manifestTarget = manifest?.targets.find(
    (entry) => targetSlug(entry) === targetSlug(downloadable),
  );
  const extension = manifestTarget?.extensions[0];
  if (!manifest || !extension) {
    throw createError({ statusCode: 404, statusMessage: `No ${channel} release found` });
  }

  // No caching directive of its own: the default is already "revalidate",
  // which is what a pointer that moves on every publish needs.
  return sendRedirect(
    event,
    versionedUrl(channel, manifest.version, downloadable.os, downloadable.arch, extension),
    302,
  );
});
