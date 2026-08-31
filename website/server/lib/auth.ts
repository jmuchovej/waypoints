import { passkey } from "@better-auth/passkey";
import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { twoFactor } from "better-auth/plugins";

import * as schema from "../database/schema";
import { db } from "./database";

const config = useRuntimeConfig();

export const auth = betterAuth({
  appName: "Waypoints",

  database: drizzleAdapter(db, { provider: "sqlite", schema }),

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
