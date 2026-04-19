import { decryptSops, findAllAgeKeys } from "sops-age";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SOPS_FILE = resolve(dirname(fileURLToPath(import.meta.url)), "secrets.sops.yaml");

// Decrypts secrets.sops.yaml at config-load time and injects values into
// process.env. Nuxt's runtimeConfig picks them up via NUXT_* env var overrides.
//
// In production, the SOPS file isn't shipped — platform env vars take over.
export async function loadSopsSecrets() {
  if (!existsSync(SOPS_FILE)) return;

  try {
    const keys = await findAllAgeKeys();
    if (keys.length === 0) {
      console.warn(
        "[secrets] no age keys found (checked SOPS_AGE_KEY, ~/.config/sops/age/keys.txt)",
      );
      return;
    }

    let decrypted: Record<string, unknown> | undefined;
    for (const key of keys) {
      try {
        decrypted = await decryptSops({ path: SOPS_FILE, secretKey: key, fileType: "yaml" });
        break;
      } catch {
        // try next key
      }
    }

    if (!decrypted) {
      console.warn(`[secrets] no available age key matched ${SOPS_FILE} recipients`);
      return;
    }

    for (const [key, value] of Object.entries(decrypted)) {
      if (value == null || value === "") continue;
      if (typeof process.env[key] !== "undefined") continue;
      process.env[key] = String(value);
    }
  } catch (err) {
    console.warn(`[secrets] failed to decrypt ${SOPS_FILE}:`, (err as Error).message);
  }
}
