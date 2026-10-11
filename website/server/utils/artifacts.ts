import type { H3Event } from "h3";

// Reads go through the Worker R2 binding, never the S3 API; CI holds the only write credential.

/** The slice of Cloudflare's `R2Bucket` this site uses, so `@cloudflare/workers-types` stays out. */
interface R2ObjectBody {
  body: ReadableStream;
  size: number;
  etag: string;
  httpEtag: string;
  uploaded: Date;
  writeHttpMetadata(headers: Headers): void;
}

interface ReleasesBucket {
  get(key: string): Promise<R2ObjectBody | null>;
}

// Absent under plain `nuxt dev`: the dev plugin populates bindings through wrangler's
// `getPlatformProxy()`, and with `wrangler` uninstalled it stubs `env` as `{}`.
export function releasesBucket(event: H3Event): ReleasesBucket | null {
  const env = (event.context.cloudflare as { env?: Record<string, unknown> } | undefined)?.env;
  const bucket = env?.RELEASES as ReleasesBucket | undefined;

  return bucket && typeof bucket.get === "function" ? bucket : null;
}

export async function readArtifact(event: H3Event, key: string): Promise<R2ObjectBody> {
  const bucket = releasesBucket(event);

  // 503 rather than 404: an unreachable bucket must not read as an unpublished release.
  if (!bucket) {
    throw createError({
      statusCode: 503,
      statusMessage:
        "Release storage is unavailable: no RELEASES R2 binding on this deployment." +
        " Local dev needs `wrangler` installed and a wrangler config for binding emulation.",
    });
  }

  const object = await bucket.get(key);

  if (!object) {
    throw createError({ statusCode: 404, statusMessage: `No such release artifact: ${key}` });
  }

  return object;
}
