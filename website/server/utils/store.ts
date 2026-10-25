import type { R2Bucket, R2ObjectBody } from "@cloudflare/workers-types";
import type { H3Event } from "h3";

import {
  manifestKey,
  versionFromKey,
  type Channel,
  type ReleaseManifest,
} from "#shared/utils/releases";

/**
 * The `releases` bucket's Worker binding — read-only by convention, never by
 * anything Cloudflare enforces on the binding itself: this file never calls
 * `put`/`delete` on it, and writes reach the bucket only through the S3
 * endpoint `xtask` uses. See the design's "Write via the S3 endpoint, read
 * via a binding" decision.
 *
 * Absent under a plain dev server — no Worker, no binding — which the design
 * accepts as a known local-dev limitation rather than something to emulate
 * here. Every caller surfaces that as a 503, distinct from a 404 for a
 * genuinely missing release.
 */
function releaseBucket(event: H3Event): R2Bucket {
  const bucket = (event.context as { cloudflare?: { env?: Record<string, unknown> } }).cloudflare
    ?.env?.RELEASES as R2Bucket | undefined;
  if (!bucket) {
    throw createError({
      statusCode: 503,
      statusMessage: "The release store is unavailable in this environment.",
    });
  }
  return bucket;
}

/** Narrows a `get()` result to the body-carrying shape — always true for a
 * plain, unconditional read, which is all this store ever issues. */
function hasBody(object: R2ObjectBody | { text?: never } | null): object is R2ObjectBody {
  return object != null && typeof (object as R2ObjectBody).text === "function";
}

export async function readManifest(
  event: H3Event,
  channel: Channel,
): Promise<ReleaseManifest | null> {
  const object = await releaseBucket(event).get(manifestKey(channel));
  if (!hasBody(object)) return null;
  return object.json<ReleaseManifest>();
}

export async function readObject(event: H3Event, key: string): Promise<R2ObjectBody | null> {
  const object = await releaseBucket(event).get(key);
  return hasBody(object) ? object : null;
}

export async function readText(event: H3Event, key: string): Promise<string | null> {
  const object = await releaseBucket(event).get(key);
  if (!hasBody(object)) return null;
  return object.text();
}

/**
 * Every version a channel's prefix holds, newest first — the store's answer
 * to "what exists", per the design's "Notes stay in the forge; enumeration
 * moves to the store" decision. Nothing here is ever deleted (see "Nothing
 * published is ever deleted"), so this walks the whole prefix rather than
 * trusting a single current pointer.
 */
export async function listVersions(event: H3Event, channel: Channel): Promise<string[]> {
  const bucket = releaseBucket(event);
  const versions = new Set<string>();
  let cursor: string | undefined;
  do {
    const page = await bucket.list({ prefix: `${channel}/`, cursor });
    for (const object of page.objects) {
      const version = versionFromKey(object.key);
      if (version) versions.add(version);
    }
    cursor = page.truncated ? page.cursor : undefined;
  } while (cursor);
  return [...versions];
}

/**
 * Streams an R2 object's body straight through as the response.
 *
 * `immutable` is for the fully-versioned route only: a version's bytes
 * never change once published, so that response can be cached for a year
 * at the edge — see the design's "`latest` is a redirect to the versioned
 * URL" decision. The `latest` alias itself must never set this; it isn't
 * even a candidate, since it never streams bytes of its own.
 */
export function sendR2Object(event: H3Event, object: R2ObjectBody, immutable = false) {
  setResponseHeaders(event, {
    "Content-Type": object.httpMetadata?.contentType ?? "application/octet-stream",
    "Content-Length": object.size.toString(),
    ETag: object.httpEtag,
    ...(immutable && { "Cache-Control": "public, max-age=31536000, immutable" }),
  });
  return object.body;
}
