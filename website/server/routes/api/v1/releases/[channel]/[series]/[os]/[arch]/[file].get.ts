import { channelOf, isChannel, seriesOf, versionFromKey } from "#shared/utils/releases";

import { readObject, sendR2Object } from "../../../../../../../../utils/store";

/**
 * Streams one fully-versioned artifact straight from the store — this
 * route's own path *is* the store key, mounted under `/api/v1/releases/`.
 * Cached as immutable: a published version's bytes never change. This is
 * the URL a `latest` alias redirects to, and the one a browser saves a
 * download under; see the design's "`latest` is a redirect to the
 * versioned URL" decision.
 *
 * Fails closed on a path whose segments disagree with each other — a
 * misfiled object is representable in the bucket but never readable
 * through here, the same rule the uploader follows when writing. See the
 * design's "Channel leads the key" decision.
 */
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel");
  const series = getRouterParam(event, "series");
  const os = getRouterParam(event, "os");
  const arch = getRouterParam(event, "arch");
  const file = getRouterParam(event, "file");

  if (!channel || !isChannel(channel) || !series || !os || !arch || !file) {
    throw createError({ statusCode: 404, statusMessage: "Not found" });
  }

  const key = `${channel}/${series}/${os}/${arch}/${file}`;
  const version = versionFromKey(key);
  if (!version || channelOf(version) !== channel || seriesOf(version) !== series) {
    throw createError({ statusCode: 404, statusMessage: "Not found" });
  }

  const object = await readObject(event, key);
  if (!object) {
    throw createError({ statusCode: 404, statusMessage: `${key} is not in the store` });
  }

  return sendR2Object(event, object, true);
});
