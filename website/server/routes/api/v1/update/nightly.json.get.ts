export default defineEventHandler(async (event) => {
  const manifest = await buildUpdateManifest(event, "nightly");

  if (!manifest) {
    setResponseStatus(event, 204);
    return null;
  }

  return manifest;
});
