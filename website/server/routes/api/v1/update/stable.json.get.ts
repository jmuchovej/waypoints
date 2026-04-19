export default defineEventHandler(async () => {
  const manifest = await buildUpdateManifest("stable");

  if (!manifest) {
    throw createError({ statusCode: 204, statusMessage: "No Content" });
  }

  return manifest;
});
