import { passkey } from "@better-auth/passkey";
import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { twoFactor } from "better-auth/plugins";
import type { H3Event } from "h3";

import { getSiteConfig } from "#site-config/server/composables";

import * as schema from "../database/schema";
import { useDb } from "./database";

/**
 * `betterAuth` is generic over its options object and the returned `Auth<O>`
 * is invariant in `O` (it exposes a `DBAdapter<O>`), so `ReturnType<typeof
 * betterAuth>` — which instantiates `O` at its `BetterAuthOptions` constraint —
 * is not a supertype of what the call below actually returns. Deriving the
 * alias from this non-generic factory keeps the concrete instantiation, which
 * is also what preserves the twoFactor/passkey endpoint and `$Infer` types.
 */
function createAuth(event?: H3Event) {
  const config = useRuntimeConfig(event);
  const siteConfig = getSiteConfig(event!);

  return betterAuth({
    appName: "Waypoints",
    baseURL: siteConfig.url,

    database: drizzleAdapter(useDb(event), { provider: "sqlite", schema }),

    emailAndPassword: {
      enabled: true,
    },

    socialProviders: {
      github: {
        clientId: config.github.clientId,
        clientSecret: config.github.clientSecret,
      },
    },

    plugins: [
      twoFactor({
        issuer: "Waypoints",
      }),
      passkey({
        rpName: "Waypoints",
      }),
    ],

    session: {
      expiresIn: 60 * 60 * 24 * 30, // 30 days
      updateAge: 60 * 60 * 24, // refresh every 24h
    },

    secrets: [{ version: 1, value: config.betterAuthSecret }],

    advanced: {
      database: {
        generateId: () => crypto.randomUUID(),
      },
    },
  });
}

type Auth = ReturnType<typeof createAuth>;

let cached: Auth | undefined;

/**
 * Built lazily for the same reason as `useDb`: on Cloudflare Workers, module
 * scope is evaluated at isolate startup, so a top-level `useRuntimeConfig()`
 * cannot see the per-request `env` bindings holding the GitHub OAuth
 * credentials and the better-auth secret. See `server/lib/database.ts`.
 */
export function useAuth(event?: H3Event): Auth {
  cached ??= createAuth(event);
  return cached;
}
