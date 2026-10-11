import {
  bundleKeyFor,
  downloadNameFor,
  isArch,
  isPlatform,
  savedNameFor,
  seriesFor,
  versionFromDownloadName,
} from "#shared/utils/distribution";
import { UPDATE_CHANNELS, type UpdateChannel, channelForVersion } from "#shared/utils/releases";

// `{bundle}` carries the version, so it is load-bearing: every check below must pass before
// the key is rebuilt from validated parts, or a request could name any object in the bucket.
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel") as UpdateChannel;
  const series = decodeURIComponent(getRouterParam(event, "series") ?? "");
  const platform = getRouterParam(event, "platform") ?? "";
  const arch = getRouterParam(event, "arch") ?? "";
  const bundle = decodeURIComponent(getRouterParam(event, "bundle") ?? "");

  if (!UPDATE_CHANNELS.includes(channel)) {
    throw createError({ statusCode: 404, statusMessage: `Unknown channel: ${channel}` });
  }

  if (!isPlatform(platform)) {
    throw createError({ statusCode: 404, statusMessage: `Unknown platform: ${platform}` });
  }

  if (!isArch(arch)) {
    throw createError({ statusCode: 404, statusMessage: `Unknown arch: ${arch}` });
  }

  const version = versionFromDownloadName(platform, bundle);

  if (!version || downloadNameFor(platform, version) !== bundle) {
    throw createError({
      statusCode: 404,
      statusMessage: `Not a ${platform} download name: ${bundle}`,
    });
  }

  if (seriesFor(version) !== series) {
    throw createError({
      statusCode: 404,
      statusMessage: `Version ${version} does not belong to series ${series}`,
    });
  }

  if (channelForVersion(version) !== channel) {
    throw createError({
      statusCode: 404,
      statusMessage: `Version ${version} is not a ${channel} release`,
    });
  }

  const key = bundleKeyFor(platform, arch, version);

  if (!key) {
    throw createError({ statusCode: 404, statusMessage: `Unparseable version: ${version}` });
  }

  const object = await readArtifact(event, key);

  const headers = new Headers();
  object.writeHttpMetadata(headers);
  for (const [name, value] of headers) setResponseHeader(event, name, value);

  // Immutable is sound because the key carries the nightly counter: a rebuild writes a new key.
  setResponseHeaders(event, {
    "Content-Disposition": `attachment; filename="${savedNameFor(platform, version)}"`,
    "Content-Length": String(object.size),
    "Cache-Control": "public, max-age=31536000, immutable",
    ETag: object.httpEtag,
  });

  return object.body;
});
