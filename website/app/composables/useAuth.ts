import { createAuthClient } from "better-auth/vue";
import { twoFactorClient } from "better-auth/client/plugins";
import { passkeyClient } from "@better-auth/passkey/client";

const authClient = createAuthClient({
  plugins: [
    twoFactorClient({
      twoFactorPage: "/auth/2fa",
    }),
    passkeyClient(),
  ],
});

export function useAuth() {
  return authClient;
}

export function useSession() {
  return authClient.useSession(useFetch);
}
