import { drizzle } from "drizzle-orm/libsql";
import type { H3Event } from "h3";

type Db = ReturnType<typeof drizzle>;

let cached: Db | undefined;

/**
 * The Turso client is built lazily, on first request, rather than at module
 * scope. On Cloudflare Workers module scope runs at isolate startup — before
 * any request exists — so `useRuntimeConfig()` there can only ever see
 * build-time values, never the per-request `env` bindings that hold
 * `wrangler secret` values.
 *
 * Caching in module scope after the first call is still correct: the isolate
 * outlives individual requests and the config does not change between them.
 */
export function useDb(event?: H3Event): Db {
  if (cached) return cached;

  const { turso } = useRuntimeConfig(event);
  if (!turso.databaseUrl) {
    throw new Error(
      "turso.databaseUrl is empty — set NUXT_TURSO_DATABASE_URL (a Worker secret in production, secrets.sops.yaml locally).",
    );
  }

  cached = drizzle({
    connection: {
      url: turso.databaseUrl,
      authToken: turso.authToken || undefined,
    },
  });
  return cached;
}
