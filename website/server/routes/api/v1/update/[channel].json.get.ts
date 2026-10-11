import { manifestKeyFor } from "#shared/utils/distribution";
import { UPDATE_CHANNELS, type UpdateChannel } from "#shared/utils/releases";

// CI publishes `{channel}.json` beside the artifacts; this route only hands it back. It is also
// the site's answer to what version a channel is on, so page and updater cannot disagree.
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel") as UpdateChannel;

  if (!UPDATE_CHANNELS.includes(channel)) {
    throw createError({ statusCode: 404, statusMessage: `Unknown channel: ${channel}` });
  }

  const object = await readArtifact(event, manifestKeyFor(channel));

  setResponseHeaders(event, {
    "Content-Type": "application/json; charset=utf-8",
    // The one mutable object in the bucket, so it must not be cached like an artifact.
    "Cache-Control": "public, max-age=300, must-revalidate",
    ETag: object.httpEtag,
  });

  return object.body;
});
