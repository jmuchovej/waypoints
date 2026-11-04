// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { auth } from "~~/server/lib/auth";

export default defineEventHandler((event) => {
  return auth.handler(toWebRequest(event));
});
