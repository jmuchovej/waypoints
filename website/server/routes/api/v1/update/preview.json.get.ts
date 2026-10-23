export default defineEventHandler(async (event) => {
  const manifest = await buildUpdateManifest(event, "preview");

  if (!manifest) {
    setResponseStatus(event, 204);
    return null;
  }

  return manifest;
});
