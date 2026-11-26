// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { isChannel, manifestKey } from "#shared/utils/releases";

import { readObject, sendR2Object } from "../../../../../utils/store";

/**
 * The channel's release manifest, streamed as the publishing run wrote it —
 * the record a package manifest derives its version, URLs, and checksums
 * from without a store credential. Not immutable: each publish replaces it.
 * The writer sets no content type, so this route names it.
 */
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel");
  if (!channel || !isChannel(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }

  const object = await readObject(event, manifestKey(channel));
  if (!object) {
    throw createError({ statusCode: 404, statusMessage: `${channel} has no published release` });
  }

  const body = sendR2Object(event, object);
  setResponseHeader(event, "Content-Type", "application/json");
  return body;
});
