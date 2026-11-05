// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

export default defineEventHandler(async (event) => {
  const manifest = await buildUpdateManifest(event, "preview");

  if (!manifest) {
    setResponseStatus(event, 204);
    return null;
  }

  return manifest;
});
