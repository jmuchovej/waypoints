import { isChannel, TAURI_UPDATER_TARGETS } from "#shared/utils/releases";
import { readObject, sendR2Object } from "../../../../../../utils/store";
import { updaterPayloadKey } from "../../../../../../utils/updater";

/**
 * Streams the updater payload for one of Tauri's own target spellings
 * (`darwin-aarch64`, `linux-x86_64`, `windows-x86_64`) — the `url` the
 * updater manifest hands the client. Separate from the installer route:
 * macOS's payload (`.app.tar.gz`) is a different object than its installer
 * (`.dmg`), even though Linux and Windows serve the very same file both
 * ways.
 */
export default defineEventHandler(async (event) => {
  const channel = getRouterParam(event, "channel");
  const target = getRouterParam(event, "target");

  if (!channel || !isChannel(channel)) {
    throw createError({ statusCode: 404, statusMessage: "Unknown channel" });
  }
  if (!target || !(target in TAURI_UPDATER_TARGETS)) {
    throw createError({ statusCode: 404, statusMessage: `Unknown updater target: ${target}` });
  }

  const key = await updaterPayloadKey(event, channel, target);
  if (!key) {
    throw createError({ statusCode: 404, statusMessage: `No ${channel} release found` });
  }

  const object = await readObject(event, key);
  if (!object) {
    throw createError({ statusCode: 404, statusMessage: `${key} is not in the store` });
  }

  return sendR2Object(event, object);
});
