export default defineEventHandler(async (event) => {
  const manifest = await buildUpdateManifest(event, "stable");

  if (!manifest) {
    setResponseStatus(event, 204);
    return null;
  }

  return manifest;
});
