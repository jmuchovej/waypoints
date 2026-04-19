import { drizzle } from "drizzle-orm/libsql";

const config = useRuntimeConfig();

export const db = drizzle({
  connection: {
    url: config.turso.databaseUrl,
    authToken: config.turso.authToken || undefined,
  },
});
