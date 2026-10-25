export default defineEventHandler(async (event) => {
  const manifest = await buildUpdateManifest("preview");

  if (!manifest) {
    setResponseStatus(event, 204);
    return null;
  }

  return manifest;
});
