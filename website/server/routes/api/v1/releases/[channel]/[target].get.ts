export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel") as "stable" | "preview";
  const target = getRouterParam(event, "target") as string;

  if (!["stable", "preview"].includes(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }

  if (!SUPPORTED_TARGETS.includes(target)) {
    throw createError({
      statusCode: 404,
      statusMessage: `Unknown target: ${target}. Supported: ${SUPPORTED_TARGETS.join(", ")}`,
    });
  }

  const releases = await fetchGitHubReleases();
  const latest = latestForChannel(releases, channel);

  if (!latest) {
    throw createError({ statusCode: 404, statusMessage: `No ${channel} release found` });
  }

  const asset = resolveDownloadAsset(latest.assets, target);

  if (!asset) {
    throw createError({
      statusCode: 404,
      statusMessage: `No asset found for ${target} in ${latest.tag}`,
    });
  }

  return sendRedirect(event, asset.browser_download_url, 302);
});
