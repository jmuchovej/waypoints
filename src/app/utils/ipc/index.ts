// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as charts from "./charts";
import * as editor from "./editor";
import * as marks from "./marks";
import * as sync from "./sync";
import * as territory from "./territory";

export { listen } from "./invoke";
export type { IPCError, Transport } from "./invoke";

export const ipc = { charts, marks, editor, sync, territory };
