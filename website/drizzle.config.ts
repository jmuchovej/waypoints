import "@types/node";
import { defineConfig } from "drizzle-kit";

export default defineConfig({
  out: "./drizzle",
  schema: "./server/database/schema.ts",
  dialect: "turso",
  dbCredentials: {
    url: process.env.NUXT_TURSO_DATABASE_URL as string,
    authToken: process.env.NUXT_TURSO_AUTH_TOKEN as string,
  },
});
