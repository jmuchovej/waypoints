// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { fileURLToPath } from "node:url";

import { defineConfig } from "vitest/config";

// Plain node tests. Server code imports shared modules through Nuxt's
// `#shared` alias, which only a Nuxt build resolves on its own.
export default defineConfig({
  resolve: {
    alias: {
      "#shared": fileURLToPath(new URL("./shared", import.meta.url)),
    },
  },
});
