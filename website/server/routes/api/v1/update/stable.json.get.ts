export default defineEventHandler(async (event) => {
  const manifest = await buildUpdateManifest("stable", event);

  if (!manifest) {
    setResponseStatus(event, 204);
    return null;
  }

  return manifest;
});
