// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { passkeyClient } from "@better-auth/passkey/client";
import { twoFactorClient } from "better-auth/client/plugins";
import { createAuthClient } from "better-auth/vue";

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
