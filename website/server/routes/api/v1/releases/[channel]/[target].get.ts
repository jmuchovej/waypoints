import { DOWNLOADABLE_TARGETS, isChannel, objectKey, targetSlug } from "#shared/utils/releases";
import { readManifest, readObject, sendR2Object } from "../../../../../utils/store";

/** Streams the current installer for `target` under `channel`, straight
 * from the store — the site never redirects to a GitHub release asset, and
 * the bucket itself stays anonymously unreadable. See task 4.1. */
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel");
  const target = getRouterParam(event, "target");

  if (!channel || !isChannel(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }

  const downloadable = DOWNLOADABLE_TARGETS.find((candidate) => targetSlug(candidate) === target);
  if (!downloadable) {
    throw createError({
      statusCode: 404,
      statusMessage: `Unknown target: ${target}. Supported: ${DOWNLOADABLE_TARGETS.map(targetSlug).join(", ")}`,
    });
  }

  const manifest = await readManifest(event, channel);
  if (!manifest) {
    throw createError({ statusCode: 404, statusMessage: `No ${channel} release found` });
  }

  const manifestTarget = manifest.targets.find((entry) => targetSlug(entry) === target);
  const installerExtension = manifestTarget?.extensions[0];
  if (!installerExtension) {
    throw createError({ statusCode: 404, statusMessage: `No ${target} build in ${manifest.version}` });
  }

  const key = objectKey(channel, manifest.version, downloadable.os, downloadable.arch, installerExtension);
  const object = await readObject(event, key);
  if (!object) {
    throw createError({ statusCode: 404, statusMessage: `${key} is not in the store` });
  }

  return sendR2Object(event, object);
});
