import { useAuth } from "~~/server/lib/auth";

export default defineEventHandler((event) => {
  return useAuth(event).handler(toWebRequest(event));
});
