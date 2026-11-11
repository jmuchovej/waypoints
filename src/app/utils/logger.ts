// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import { createConsola, LogLevels } from "consola";

export function createLogger(tag: string) {
  return createConsola({
    level: import.meta.dev ? LogLevels.debug : LogLevels.info,
  }).withTag(tag);
}
